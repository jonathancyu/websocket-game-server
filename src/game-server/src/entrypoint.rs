use super::model::internal::GameRequest;
use super::service::game_manager::GameManager;
use super::service::game_socket::GameSocket;
use common::utility::random_address;
use common::websocket::WebsocketHandler;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, Level};

pub async fn serve(
    manager_address: String,
    socket_address: String,
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
    let manager_handle = tokio::spawn(async move {
        GameManager::new()
            .run(
                manager_address,
                &mut manager_shutdown_receiver,
                to_game_receiver,
            )
            .await
    });
    // Websocket handler - route client to its corresponding game
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        GameSocket::new()
            .listen(socket_address, &mut game_shutdown_receiver, to_game_sender)
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
        .expect("Websocket exited non-gracefully");
}

pub struct GameServer {
    pub manager_address: String,
    pub socket_address: String,
    shutdown_sender: broadcast::Sender<()>,
}
impl GameServer {
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
        GameServer {
            shutdown_sender,
            manager_address,
            socket_address,
        }
    }
    pub async fn shutdown(&self) {
        self.shutdown_sender.send(()).expect("Failed to shutdown");
    }
}
