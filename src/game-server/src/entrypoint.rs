use super::model::internal::GameRequest;
use super::service::game_manager::GameManager;
use super::service::game_socket::GameSocket;
use common::websocket::WebsocketHandler;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, Level};

#[derive(Clone)]
pub struct GameServerConfig {
    pub manager_address: String,
    pub socket_address: String,
}

pub async fn serve(
    config: GameServerConfig,
    shutdown_receiver: tokio::sync::broadcast::Receiver<()>,
    ready_signal: Option<tokio::sync::oneshot::Sender<()>>,
) {
    let mut game_shutdown_receiver = shutdown_receiver.resubscribe();
    let mut manager_shutdown_receiver = shutdown_receiver.resubscribe();
    let (to_game_sender, to_game_receiver): (
        mpsc::Sender<GameRequest>,
        mpsc::Receiver<GameRequest>,
    ) = mpsc::channel(100);

    // REST endpoint: listen for game creation signals from central server
    // One thread per game
    let manager_config = config.clone();
    let manager_handle = tokio::spawn(async move {
        GameManager::new()
            .run(
                manager_config,
                &mut manager_shutdown_receiver,
                to_game_receiver,
            )
            .await
    });
    // Websocket handler - route client to its corresponding game
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        GameSocket::new()
            .listen(
                config.socket_address,
                &mut game_shutdown_receiver,
                to_game_sender,
            )
            .await
    });
    // Signal that the server is ready
    ready_signal
        .unwrap()
        .send(())
        .expect("Failed to send ready signal");

    manager_handle
        .await
        .expect("Game manager exited non-gracefully");
    websocket_handle
        .await
        .expect("Websocket exited non-gracefully");
}

pub struct GameServer {
    pub config: GameServerConfig,
    shutdown_sender: broadcast::Sender<()>,
}
impl GameServer {
    pub async fn new(config: GameServerConfig) -> Self {
        // Init logging, ignore error if already set
        let _ = tracing_subscriber::fmt()
            .with_line_number(true)
            .with_file(true)
            .with_max_level(Level::DEBUG)
            .try_init();

        // Create server
        let (shutdown_sender, shutdown_receiver) = tokio::sync::broadcast::channel(1);
        let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();

        tokio::spawn(serve(config.clone(), shutdown_receiver, Some(ready_sender)));

        // Wait for server to be ready
        ready_receiver.await.expect("Server failed to start");

        // Return server
        GameServer {
            config,
            shutdown_sender,
        }
    }
    pub async fn shutdown(&self) {
        self.shutdown_sender.send(()).expect("Failed to shutdown");
    }
}
