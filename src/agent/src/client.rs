use common::model::game::{Move, Outcome};

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

    pub fn play(&mut self) -> Move {
        let next_move = self.strategy.make_move(&self.history);
        self.last_move = Some(next_move.clone());
        next_move
    }

    pub fn record_round(&mut self, their_move: Move, outcome: Outcome) {
        if let Some(my_move) = self.last_move.take() {
            self.history.push(Round {
                my_move,
                their_move,
                outcome,
            });
        }
    }

    pub fn strategy_name(&self) -> &'static str {
        self.strategy.name()
    }
}
