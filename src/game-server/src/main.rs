use common::utility::{create_shutdown_channel, Channel};
use common::websocket::WebsocketHandler;
use game_server::service::game_manager::GameManager;
use game_server::service::game_socket::GameSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, Level};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();
    let shutdown_receiver = create_shutdown_channel().await;
    let manager_address = "0.0.0.0:8080".to_owned();
    let socket_address = "0.0.0.0:3002".to_owned();
    serve(manager_address, socket_address, shutdown_receiver, None).await;
}
async fn serve(
    manager_address: String,
    socket_address: String,
    shutdown_receiver: tokio::sync::broadcast::Receiver<()>,
    ready_signal: Option<tokio::sync::oneshot::Sender<()>>,
) {
    let mut game_shutdown_receiver = shutdown_receiver.resubscribe();
    let to_game_channel = Channel::from(mpsc::channel(100));

    // REST endpoint: listen for game creation signals from central server
    // One thread per game
    let manager_handle =
        tokio::spawn(async move { GameManager::new().listen(manager_address).await });
    // Websocket handler - route client to its corresponding game
    // TODO: one thread per game
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        GameSocket::new()
            .listen(
                socket_address,
                &mut game_shutdown_receiver,
                to_game_channel.sender,
            )
            .await
    });
    // Signal that the server is ready
    if let Some(ready_signal) = ready_signal {
        info!("Sent ready");
        ready_signal.send(()).expect("Failed to send ready signal");
    }

    manager_handle
        .await
        .expect("Game manager exited non-gracefully");

    websocket_handle
        .await
        .expect("Socket thread exited non-gracefully");
}

// reference: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
// https://github.com/tokio-rs/axum/blob/main/examples/reqwest-response/src/main.rs
#[cfg(test)]
mod tests {
    use std::fs;

    use common::{
        model::messages::{CreateGameRequest, Id, SocketRequest},
        websocket::test::TestCase,
    };
    use futures_util::{SinkExt, StreamExt};
    use game_server::model::external::{ClientRequest, ClientResponse};
    use reqwest::{Client, StatusCode};
    use serde_json::json;
    use tokio::{net::UdpSocket, sync::broadcast};
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use tracing::debug;
    use tracing_subscriber::util::SubscriberInitExt;
    use uuid::Uuid;

    use super::*;
    struct TestServer {
        pub manager_address: String,
        pub socket_address: String,
        shutdown_sender: broadcast::Sender<()>,
    }
    impl TestServer {
        pub async fn new() -> Self {
            // Init logging, ignore error if already set
            let _ = tracing_subscriber::fmt()
                .with_line_number(true)
                .with_file(true)
                .with_max_level(Level::DEBUG)
                .try_init();

            // Create server
            let (shutdown_sender, shutdown_receiver) = tokio::sync::broadcast::channel(1);
            let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();

            let manager_address = random_address().await;
            let socket_address = random_address().await;
            tokio::spawn(serve(
                manager_address.clone(),
                socket_address.clone(),
                shutdown_receiver,
                Some(ready_sender),
            ));

            // Wait for server to be ready
            ready_receiver.await.expect("Server failed to start");

            // Return server
            TestServer {
                shutdown_sender,
                manager_address,
                socket_address,
            }
        }
        pub async fn shutdown(&self) {
            self.shutdown_sender.send(()).expect("Failed to shutdown");
        }
    }

    async fn random_address() -> String {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("Failed to get random port");
        socket
            .local_addr()
            .expect("Failed to unwrap local address")
            .to_string()
    }
    fn endpoint(protocol: String, base_url: String, endpoint: String) -> String {
        format!("{}://{}/{}", protocol, base_url, endpoint)
    }

    #[tokio::test]
    async fn serves_hello_world() {
        // Given
        let server = TestServer::new().await;

        // When
        let response = Client::new()
            .get(endpoint(
                "http".to_string(),
                server.manager_address.clone(),
                "".to_string(),
            ))
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
        // Given
        let server = TestServer::new().await;

        // When
        let request = CreateGameRequest {
            game_id: Id::new(),
            players: vec![Id::new(), Id::new()],
        };
        let response = Client::new()
            .post(endpoint(
                "http".to_string(),
                server.manager_address.clone(),
                "create_game".to_string(),
            ))
            .json(&request)
            .send()
            .await
            .inspect_err(|e| eprintln!("{}", e))
            .expect("Request failed");

        // Then
        assert_eq!(response.status(), StatusCode::CREATED);
        debug!("Response: {:?}", response.text().await);

        server.shutdown().await;
    }

    #[tokio::test]
    async fn simulate_full_game() {
        let server = TestServer::new().await;

        // Create game
        let player_1 = Id::new();
        let player_2 = Id::new();
        let request = CreateGameRequest {
            game_id: Id::new(),
            players: vec![player_1.clone(), player_2.clone()],
        };
        let response = Client::new()
            .post(endpoint(
                "http".to_string(),
                server.manager_address.clone(),
                "create_game".to_string(),
            ))
            .json(&request)
            .send()
            .await
            .inspect_err(|e| eprintln!("{}", e))
            .expect("Request failed");
        assert_eq!(response.status(), StatusCode::CREATED);

        // Make each player join
        let (ws_stream_1, _) = connect_async(format!("ws://{}", server.socket_address.clone()))
            .await
            .unwrap();
        let (mut write_1, mut read_1) = ws_stream_1.split();
        let req = SocketRequest {
            user_id: Some(player_1),
            request: ClientRequest::JoinGame,
        };
        let body: String = json!(req).to_string();
        write_1.send(Message::text(body)).await.unwrap();
        let resp = tokio::time::timeout(tokio::time::Duration::from_secs(1), read_1.next())
            .await
            .expect("Response timed out");
        println!("socket resp {:?}", resp);
    }
    #[tokio::test]
    async fn run_game() {
        let data_path = env!("CARGO_MANIFEST_DIR").to_string() + "/tests/data/full_game.json";
        let text = fs::read_to_string(data_path).expect("Unable to read file");
        let test_case: TestCase<ClientRequest, ClientResponse> =
            serde_json::from_str(&text).expect("Could not parse test case");
    }
}
