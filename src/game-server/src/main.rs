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
    use reqwest::{Client, StatusCode};
    use tokio::{net::UdpSocket, sync::broadcast};
    use tracing::debug;

    use super::*;
    struct TestServer {
        pub manager_address: String,
        pub socket_address: String,
        shutdown_sender: broadcast::Sender<()>,
    }
    impl TestServer {
        pub async fn new() -> Self {
            // Init logging
            tracing_subscriber::fmt()
                .with_line_number(true)
                .with_file(true)
                .with_max_level(Level::DEBUG)
                .init();

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
    fn endpoint(base_url: String, endpoint: String) -> String {
        format!("http://{}/{}", base_url, endpoint)
    }

    #[tokio::test]
    async fn can_create_game() {
        // Given
        let server = TestServer::new().await;

        // When
        let response = Client::new()
            .get(endpoint(server.manager_address.clone(), "".to_string()))
            .send()
            .await
            .inspect_err(|e| eprintln!("{}", e))
            .expect("Request failed");

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        debug!("Response: {:?}", response.text().await);

        server.shutdown().await;
    }
}
