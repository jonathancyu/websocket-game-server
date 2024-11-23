use std::sync::Arc;

use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

pub struct Channel<T> {
    pub sender: Arc<Mutex<Sender<T>>>,
    pub receiver: Arc<Mutex<Receiver<T>>>,
}
impl<T> From<(Sender<T>, Receiver<T>)> for Channel<T> {
    fn from((sender, receiver): (Sender<T>, Receiver<T>)) -> Self {
        Channel {
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}
