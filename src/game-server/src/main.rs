use common::utility::{create_shutdown_channel, Channel};
use common::websocket::WebsocketHandler;
use game_server::service::game_socket::GameSocket;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();
    let shutdown_receiver = create_shutdown_channel().await;
    let mut game_shutdown_receiver = shutdown_receiver.resubscribe();
    let to_game_channel = Channel::from(mpsc::channel(100));
    // TODO:
    // REST endpoint: listen for game creation signals from central server
    // One thread per game

    // Websocket handler - route client to its corresponding game
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        GameSocket::new()
            .listen(
                "0.0.0.0:3002".to_owned(),
                &mut game_shutdown_receiver,
                to_game_channel.sender,
            )
            .await
    });
    websocket_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
}
