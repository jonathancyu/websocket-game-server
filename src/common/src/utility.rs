use std::sync::Arc;

use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
    Mutex,
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
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
        shutdown_sender
            .send(())
            .expect("Failed to send shutdown signal");
    });
    shutdown_receiver
}
