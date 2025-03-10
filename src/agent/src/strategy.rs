use common::model::game::{Move, Outcome};

pub struct Round {
    my_move: Move,
    their_move: Move,
    outcome: Outcome,
}
pub trait Strategy {
    fn make_move(&self, history: &Vec<Round>) -> Move;
}

// Trivial strategies
pub struct OnlyRock {}
impl Strategy for OnlyRock {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Rock
    }
}
pub struct OnlyPaper {}
impl Strategy for OnlyPaper {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Paper
    }
}
pub struct OnlyScissors {}
impl Strategy for OnlyScissors {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Scissors
    }
}

// Random
pub struct RandomMove {}
impl Strategy for RandomMove {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Rock
    }
}
