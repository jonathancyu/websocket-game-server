use common::utility::create_shutdown_channel;
use matchmaking_server::entrypoint::{self, MatchmakingConfig};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();
    // Config
    let config = MatchmakingConfig {
        socket_address: "0.0.0.0:3001".to_owned(),
        rest_address: "0.0.0.0:8081".to_owned(),
        game_server_url: "http://0.0.0.0:8082".to_owned(),
        db_url: "matchmaking.db".to_owned(),
    };
    let shutdown_receiver = create_shutdown_channel().await;
    entrypoint::serve(config, shutdown_receiver, None).await;
}

#[cfg(test)]
mod tests {
    use common::test::DummyType;
    use common::{
        model::messages::Id,
        test::{ServerAddress, TestCase},
    };
    use entrypoint::MatchmakingServer;
    use game_server::entrypoint::{GameServer, GameServerConfig};
    use matchmaking_server::model::messages::{ClientRequest, ClientResponse};
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::fs;
    use tokio::net::UdpSocket;
    use tracing::debug;

    use super::*;
    async fn random_address() -> String {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("Failed to get random port");
        socket
            .local_addr()
            .expect("Failed to unwrap local address")
            .to_string()
    }

    fn url<A, B, C>(protocol: A, base_url: B, endpoint: C) -> String
    where
        A: ToString,
        B: ToString,
        C: ToString,
    {
        format!(
            "{}://{}/{}",
            protocol.to_string(),
            base_url.to_string(),
            endpoint.to_string()
        )
    }

    async fn init_test_db() -> String {
        // Use a temporary file instead of :memory: so it can be shared between connections
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("matchmaking_test_{}.db", Id::new()));
        let db_url = db_path.to_str().unwrap().to_string();

        let conn = Connection::open(&db_url).expect("Failed to create test database");

        // Read and execute the SQL schema
        let schema = fs::read_to_string(
            env!("CARGO_MANIFEST_DIR").to_string() + "/../../sql/create_tables.sql",
        )
        .expect("Failed to read schema file");

        conn.execute_batch(&schema)
            .expect("Failed to initialize database schema");

        db_url
    }

    #[tokio::test]
    async fn run_game() {
        // Initialize test database with schema
        let db_url = init_test_db().await;

        // Stand up servers
        let gs_config = GameServerConfig {
            manager_address: random_address().await.to_string(),
            socket_address: url("ws", random_address().await, ""),
        };
        let mm_config = MatchmakingConfig {
            socket_address: random_address().await,
            rest_address: random_address().await,
            game_server_url: url("http", gs_config.manager_address.clone(), ""), // TODO:
            // should probably use a Url object for this field
            db_url,
        };
        let mm_server = MatchmakingServer::new(mm_config).await;

        let game_server = GameServer::new(gs_config.clone()).await;

        // Set up test case
        let file_path =
            env!("CARGO_MANIFEST_DIR").to_string() + "/test/data/queue_multiple_times.json";
        let ids = [Id::new(), Id::new()];
        let replacements: Vec<(String, String)> = vec![
            ("user1".to_string(), ids[0].to_string()),
            ("user2".to_string(), ids[1].to_string()),
            ("game_id".to_string(), Id::new().to_string()),
            (
                "game_server_address".to_string(),
                gs_config.socket_address.clone(),
            ),
        ];
        let test_case = TestCase::<ClientRequest, ClientResponse, DummyType, DummyType>::load(
            file_path,
            replacements,
        );

        let address_lookup = HashMap::from([
            (
                "user1".to_string(),
                ServerAddress::WebSocket(url("ws", mm_server.config.socket_address.clone(), "")),
            ),
            (
                "user2".to_string(),
                ServerAddress::WebSocket(url("ws", mm_server.config.socket_address.clone(), "")),
            ),
            (
                "rest".to_string(),
                ServerAddress::RestApi(url("http", mm_server.config.rest_address.clone(), "")),
            ),
        ]);

        test_case.run(address_lookup).await;
        mm_server.shutdown().await;
        game_server.shutdown().await;
    }
}
