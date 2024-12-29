use common::utility::create_shutdown_channel;
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .with_max_level(Level::DEBUG)
        .init();
    let shutdown_receiver = create_shutdown_channel().await;
    // TODO:
    // REST endpoint: listen for game creation signals from central server
    // One thread per game

    // Websocket handler - route client to its corresponding game
}
