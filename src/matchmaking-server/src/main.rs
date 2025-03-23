use common::utility::{create_shutdown_channel, Channel};
use common::websocket::WebsocketHandler;
use matchmaking_server::service::matchmaking::MatchmakingConfig;
use matchmaking_server::service::{matchmaking::MatchmakingService, queue_socket::QueueSocket};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{info, Level};

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
    serve(config, shutdown_receiver, None).await;
}

async fn serve(
    config: MatchmakingConfig,
    shutdown_receiver: tokio::sync::broadcast::Receiver<()>,
    ready_signal: Option<tokio::sync::oneshot::Sender<()>>,
) {
    // Channels for communication between matchmaker and websockets
    let to_mm_channel = Channel::from(mpsc::channel(100));
    // Shutdown hook
    let mut mm_shutdown_receiver = shutdown_receiver.resubscribe();
    let mut ws_shutdown_receiver = shutdown_receiver.resubscribe();

    // Spawn thread for matchmaking
    let config_mm = config.clone();
    let matchmaker_handle: JoinHandle<()> = tokio::spawn(async move {
        MatchmakingService::new()
            .run(config_mm, &mut mm_shutdown_receiver, to_mm_channel.receiver)
            .await
    });
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        QueueSocket::new()
            .listen(
                config.socket_address.clone(),
                &mut ws_shutdown_receiver,
                to_mm_channel.sender,
            )
            .await
    });

    // Signal that the server is ready
    if let Some(ready_signal) = ready_signal {
        info!("Sent ready");
        ready_signal.send(()).expect("Failed to send ready signal");
    }

    websocket_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
    matchmaker_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
}

#[cfg(test)]
mod tests {
    use common::test::DummyType;
    use common::{
        model::messages::Id,
        test::{ServerAddress, TestCase},
    };
    use matchmaking_server::model::messages::{ClientRequest, ClientResponse};
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::fs;
    use tokio::{net::UdpSocket, sync::broadcast};
    use tracing::debug;

    use super::*;
    struct TestServer {
        pub rest_address: String,
        pub socket_address: String,
        shutdown_sender: broadcast::Sender<()>,
    }
    impl TestServer {
        pub async fn new(config: MatchmakingConfig) -> Self {
            // Init logging, ignore error if already set
            let _ = tracing_subscriber::fmt()
                .with_line_number(true)
                .with_file(true)
                .with_max_level(Level::DEBUG)
                .try_init();

            // Create server
            let (shutdown_sender, shutdown_receiver) = tokio::sync::broadcast::channel(1);
            let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel::<()>();

            let moved_cfg = config.clone();
            tokio::spawn(serve(moved_cfg, shutdown_receiver, Some(ready_sender)));

            // Wait for server to be ready
            ready_receiver.await.expect("Server failed to start");

            // Return server
            TestServer {
                shutdown_sender,
                rest_address: config.rest_address,
                socket_address: config.socket_address,
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
        let rest_address = random_address().await;
        let socket_address = random_address().await;
        let game_server_url = "http://0.0.0.0:8082".to_owned();

        // Initialize test database with schema
        let db_url = init_test_db().await;
        debug!("Using test database at: {}", db_url);

        let config = MatchmakingConfig {
            socket_address: socket_address.clone(),
            rest_address: rest_address.clone(),
            game_server_url: game_server_url.clone(),
            db_url,
        };

        let server = TestServer::new(config).await;
        let file_path =
            env!("CARGO_MANIFEST_DIR").to_string() + "/test/data/queue_multiple_times.json";
        let ids = [Id::new(), Id::new()];
        let replacements: Vec<(String, String)> = vec![
            ("user1".to_string(), ids[0].to_string()),
            ("user2".to_string(), ids[1].to_string()),
            ("game_id".to_string(), Id::new().to_string()),
            ("server_address".to_string(), game_server_url),
        ];
        let test_case = TestCase::<ClientRequest, ClientResponse, DummyType, DummyType>::load(
            file_path,
            replacements,
        );

        let address_lookup = HashMap::from([
            (
                "user1".to_string(),
                ServerAddress::WebSocket(url("ws", server.socket_address.clone(), "")),
            ),
            (
                "user2".to_string(),
                ServerAddress::WebSocket(url("ws", server.socket_address.clone(), "")),
            ),
            (
                "rest".to_string(),
                ServerAddress::RestApi(url("http", server.rest_address.clone(), "")),
            ),
        ]);
        test_case.run(address_lookup).await;
        server.shutdown().await;
    }
}
