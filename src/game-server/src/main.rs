use common::utility::{create_shutdown_channel, Channel};
use common::websocket::WebsocketHandler;
use game_server::service::game_manager::GameManager;
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
    serve().await;
}

async fn serve() {
    let shutdown_receiver = create_shutdown_channel().await;
    let mut game_shutdown_receiver = shutdown_receiver.resubscribe();
    let to_game_channel = Channel::from(mpsc::channel(100));

    // REST endpoint: listen for game creation signals from central server
    // One thread per game
    let manager_handle =
        tokio::spawn(async move { GameManager::new().listen("0.0.0.0:3031".to_owned()).await });
    manager_handle
        .await
        .expect("Game manager exited non-gracefully");

    // Websocket handler - route client to its corresponding game
    // TODO: one thread per game
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
        .expect("Socket thread exited non-gracefully");
}

// reference: https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs
// https://github.com/tokio-rs/axum/blob/main/examples/reqwest-response/src/main.rs
#[cfg(test)]
mod tests {
    use reqwest::{Client, StatusCode};

    use super::*;

    #[tokio::test]
    async fn can_create_game() {
        // Given
        serve().await;
        let client = Client::new();
        // When
        let response = client
            .get("http://0.0.0.0/3000")
            .send()
            .await
            .expect("Request failed");
        // Then
        assert_eq!(response.status(), StatusCode::OK);
    }
}
