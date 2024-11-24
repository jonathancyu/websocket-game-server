use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use tokio::sync::{
    broadcast,
    mpsc::{Receiver, Sender},
    Mutex,
};
use tracing::{error, info, warn};

use crate::model::messages::{MatchmakingRequest, MatchmakingResponse, QueuedPlayer, UserId};

// Structure to represent a player in queue

pub struct MatchmakingService {
    queue: VecDeque<QueuedPlayer>,
    users_in_queue: HashSet<UserId>,
}

#[derive(Debug)]
enum MatchmakingError {
    Error,
}

impl MatchmakingService {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            users_in_queue: HashSet::new(),
        }
    }
    pub fn add_user(&mut self, player: QueuedPlayer) -> Result<(), &'static str> {
        let user_id = player.clone().id;
        if !self.users_in_queue.contains(&user_id) {
            info!("Adding user {:?} to queue", user_id);
            self.queue.push_back(player);
            self.users_in_queue.insert(user_id);
            Ok(())
        } else {
            Err("User already in queue")
        }
    }

    pub async fn listen(
        &mut self,
        shutdown_receiver: &mut broadcast::Receiver<()>,
        ws_sender: Sender<MatchmakingResponse>,
        ws_receiver: Arc<Mutex<Receiver<MatchmakingRequest>>>,
    ) {
        let mut receiver = ws_receiver.lock().await;
        info!("Initialized matchmaking service");
        loop {
            // Listen for queue messages / shutdown signal
            tokio::select! {
                _ = shutdown_receiver.recv() => {
                    break
                }
                message = receiver.recv() => {
                    info!("Handling");
                    self.handle_message(message).await;
                }
            }
        }
        info!("Exiting matchmaking service");
    }

    async fn handle_message(&mut self, message: Option<MatchmakingRequest>) {
        let Some(message) = message else {
            info!("Got empty message");
            return;
        };
        match message {
            MatchmakingRequest::JoinQueue(player) => {
                let sender = player.sender.clone();
                if sender.is_closed() {
                    warn!("Sender {:?} is closed!", player.id);
                }
                let _ = self.add_user(player);
                let result = sender.send(MatchmakingResponse::QueueJoined).await;
                if let Err(err) = result {
                    error!("Got error when sending MatchmakingResponse: {}", err);
                }
            }
            MatchmakingRequest::LeaveQueue(user_id) => {
                match self.users_in_queue.get(&user_id) {
                    Some(_) => {
                        let position = self.queue.iter().enumerate().find(|(_, user)| user.id == user_id);
                        if let Some((position, user)) = position {
                            info!("Removing user {:?} from queue", user.id);
                            self.queue.remove(position);
                        } else {
                            warn!(
                                "User {:?} was in users_in_queue but not in actual queue",
                                user_id
                            );
                        }
                    }
                    None => warn!("User {:?} not in queue", user_id),
                }
                if self.users_in_queue.contains(&user_id) {}
            }
            MatchmakingRequest::Disconnected(user_id) => {
                warn!("User {:?} disconnected. What to do..?", user_id);
            }
        };
    }
}

impl Default for MatchmakingService {
    fn default() -> Self {
        Self::new()
    }
}