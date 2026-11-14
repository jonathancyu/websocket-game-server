use std::sync::Arc;

use tokio::{
    net::UdpSocket,
    signal::{self},
    sync::{
        broadcast,
        mpsc::{Receiver, Sender},
        Mutex,
    },
};

#[derive(Clone)]
pub struct Channel<T> {
    pub sender: Sender<T>,
    pub receiver: Arc<Mutex<Receiver<T>>>,
}
impl<T> From<(Sender<T>, Receiver<T>)> for Channel<T> {
    fn from((sender, receiver): (Sender<T>, Receiver<T>)) -> Self {
        Channel {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

pub async fn create_shutdown_channel() -> broadcast::Receiver<()> {
    let (shutdown_sender, shutdown_receiver): (broadcast::Sender<()>, broadcast::Receiver<()>) =
        broadcast::channel::<()>(100);
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::spawn(async move {
        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
        shutdown_sender
            .send(())
            .expect("Failed to send shutdown signal");
    });
    shutdown_receiver
}

/// Binds an ephemeral UDP port and returns its local address as a string.
pub async fn random_address() -> String {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .expect("Failed to get random port");
    socket
        .local_addr()
        .expect("Failed to unwrap local address")
        .to_string()
}
/// Builds a URL of the form `protocol://base_url/endpoint`.
pub fn url<A, B, C>(protocol: A, base_url: B, endpoint: C) -> String
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_joins_parts() {
        assert_eq!(url("http", "localhost:8080", "games"), "http://localhost:8080/games");
    }

    #[test]
    fn url_with_empty_endpoint_keeps_trailing_slash() {
        assert_eq!(url("ws", "127.0.0.1:1", ""), "ws://127.0.0.1:1/");
    }

    #[test]
    fn url_accepts_any_to_string() {
        assert_eq!(url("http", "host", 42), "http://host/42");
    }
}
