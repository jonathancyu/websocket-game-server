use common::utility::Channel;
use common::websocket::WebsocketHandler;
use server::service::{matchmaking::MatchmakingService, queue_socket::QueueSocket};
use tokio::sync::broadcast;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();

    // Channels for communication between matchmaker and websockets
    let to_mm_channel = Channel::from(mpsc::channel(100));
    // Shutdown hook
    let (shutdown_sender, shutdown_receiver) = broadcast::channel::<()>(100);
    let mut mm_shutdown_receiver = shutdown_receiver.resubscribe();
    let mut ws_shutdown_receiver = shutdown_receiver.resubscribe();

    // Wait for ctrl-c to gracefully exit
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
        shutdown_sender
            .send(())
            .expect("Failed to send shutdown signal");
    });
    // Spawn thread for matchmaking
    let matchmaker_handle: JoinHandle<()> = tokio::spawn(async move {
        MatchmakingService::new()
            .listen(&mut mm_shutdown_receiver, to_mm_channel.receiver)
            .await;
    });
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        QueueSocket::new()
            .listen(
                "0.0.0.0:3001".to_owned(),
                &mut ws_shutdown_receiver,
                to_mm_channel.sender,
            )
            .await;
    });

    websocket_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
    matchmaker_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
}
