use std::collections::HashSet;

use tracing::info;
use uuid::Uuid;

use crate::model::messages::{QueueMessage, UserId};

pub struct MatchmakingService {
    queue: Vec<User>,
    users_in_queue: HashSet<UserId>,
}
struct User {
    user_id: UserId,
}

impl MatchmakingService {
    pub fn new() -> Self {
        MatchmakingService {
            queue: vec![],
            users_in_queue: HashSet::new(),
        }
    }
    pub fn add_user(&mut self, message: QueueMessage) -> Result<(), &'static str> {
        let user_id = message.user_id;
        if !self.users_in_queue.contains(&user_id) {
            info!("Adding user {:?} to queue", user_id);
            self.queue.push(User { user_id });
            Ok(())
        } else {
            Err("User already in queue")
        }
    }
}
