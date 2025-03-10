use common::model::game::Move;

use crate::strategy::{Round, Strategy};

pub struct Client {
    strategy: Box<dyn Strategy>,
    history: Vec<Round>,
    last_move: Option<Move>,
}

impl Client {
    pub fn new(strategy: Box<dyn Strategy>) -> Self {
        Client {
            strategy,
            history: Vec::new(),
            last_move: None,
        }
    }

    fn play(&mut self) -> Move {
        let next_move = self.strategy.make_move(&self.history);
        self.last_move = Some(next_move.clone());
        next_move
    }
}
