use common::utility::create_shutdown_channel;
use game_server::entrypoint::{self, GameServerConfig};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();
    let shutdown_receiver = create_shutdown_channel().await;
    let config = GameServerConfig {
        manager_address: "0.0.0.0:8082".to_owned(),
        socket_address: "0.0.0.0:3002".to_owned(),
    };
    entrypoint::serve(config, shutdown_receiver, None).await;
}

/// TODO: Does this belong in tests/?
// reference: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
// https://github.com/tokio-rs/axum/blob/main/examples/reqwest-response/src/main.rs
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use common::message::game_server::{ClientRequest, ClientResponse};
    use common::reqwest::{Client, StatusCode};
    use common::utility::{random_address, url};
    use common::{
        model::messages::{
            CreateGameRequest, CreateGameResponse, GetGameRequest, GetGameResponse, Id,
        },
        test::{ServerAddress, TestCase},
    };
    use entrypoint::GameServer;
    use tracing::debug;

    use super::*;
    async fn make_config() -> GameServerConfig {
        GameServerConfig {
            manager_address: random_address().await.to_string(),
            socket_address: random_address().await.to_string(),
        }
    }

    #[tokio::test]
    async fn serves_hello_world() {
        // Given
        let server = GameServer::new(make_config().await).await;

        // When
        let response = Client::new()
            .get(url("http", server.config.manager_address.clone(), ""))
            .send()
            .await
            .inspect_err(|e| eprintln!("{}", e))
            .expect("Request failed");

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        debug!("Response: {:?}", response.text().await);

        server.shutdown().await;
    }
    #[tokio::test]
    async fn can_create_game() {
        let server = GameServer::new(make_config().await).await;

        // POST game
        let request = CreateGameRequest {
            players: vec![Id::new(), Id::new()],
            games_to_win: 3,
        };
        let client = Client::new();
        let response = client
            .post(url(
                "http",
                server.config.manager_address.clone(),
                "create_game",
            ))
            .json(&request)
            .send()
            .await
            .expect("Request failed");
        assert_eq!(StatusCode::CREATED, response.status());

        let game = response
            .json::<CreateGameResponse>()
            .await
            .expect("Failed to get create game response body");
        let game_id = game.game_id;

        // GET game
        let get_url = url(
            "http",
            server.config.manager_address.clone(),
            format!("game/{}", &game_id.to_string()),
        );
        let response = client
            .get(dbg!(get_url))
            .json(&GetGameRequest { game_id })
            .send()
            .await
            .expect("Request failed");
        assert_eq!(StatusCode::OK, response.status());

        let game = response
            .json::<GetGameResponse>()
            .await
            .expect("Failed to get game response body");
        assert_eq!(game_id, game.game_id);

        server.shutdown().await;
    }

    #[tokio::test]
    async fn run_game() {
        let config = make_config().await;
        let server = GameServer::new(config.clone()).await;
        let file_path = env!("CARGO_MANIFEST_DIR").to_string() + "/test/data/full_game.json";
        let ids = [Id::new(), Id::new()];
        let replacements = vec![
            ("user1", ids[0].to_string()),
            ("user2", ids[1].to_string()),
            ("socket_address", config.socket_address.to_string()),
        ];
        let test_case =
            TestCase::<ClientRequest, ClientResponse, CreateGameRequest, CreateGameResponse>::load(
                file_path,
                replacements,
            );

        let address_lookup = HashMap::from([
            (
                "user1".to_string(),
                ServerAddress::WebSocket(url("ws", server.config.socket_address.clone(), "")),
            ),
            (
                "user2".to_string(),
                ServerAddress::WebSocket(url("ws", server.config.socket_address.clone(), "")),
            ),
            (
                "rest".to_string(),
                ServerAddress::RestApi(url("http", server.config.manager_address.clone(), "")),
            ),
        ]);
        test_case.run(address_lookup).await;
    }
}
