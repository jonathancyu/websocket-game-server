use super::service::{matchmaking::MatchmakingService, queue_socket::QueueSocket};
use common::utility::Channel;
use common::websocket::WebsocketHandler;
use tokio::sync::broadcast;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{info, Level};

#[derive(Clone)]
pub struct MatchmakingConfig {
    pub socket_address: String,
    pub rest_address: String,
    pub db_url: String,
    pub game_server_url: String,
}

pub async fn serve(
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

pub struct MatchmakingServer {
    pub config: MatchmakingConfig,
    shutdown_sender: broadcast::Sender<()>,
}

impl MatchmakingServer {
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
        MatchmakingServer {
            shutdown_sender,
            config,
        }
    }
    pub async fn shutdown(&self) {
        self.shutdown_sender.send(()).expect("Failed to shutdown");
    }
}
