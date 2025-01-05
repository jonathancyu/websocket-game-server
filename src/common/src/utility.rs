use std::sync::Arc;

use tokio::{
    signal::{self, unix::signal},
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

// TODO: consolodate with above
// Source: https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
pub async fn shutdown_signal() {}
