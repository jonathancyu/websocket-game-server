use common::model::game::{Move, Outcome};

/// One completed round, kept so strategies can inspect history.
pub struct Round {
    my_move: Move,
    their_move: Move,
    outcome: Outcome,
}
/// A policy that picks the next move given the history so far.
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

// Random (TODO: currently always plays Rock; needs an RNG dependency)
pub struct RandomMove {}
impl Strategy for RandomMove {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Rock
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_history() -> Vec<Round> {
        vec![Round {
            my_move: Move::Rock,
            their_move: Move::Paper,
            outcome: Outcome::Loss,
        }]
    }

    #[test]
    fn trivial_strategies_ignore_history() {
        for history in [Vec::new(), sample_history()] {
            assert_eq!(OnlyRock {}.make_move(&history), Move::Rock);
            assert_eq!(OnlyPaper {}.make_move(&history), Move::Paper);
            assert_eq!(OnlyScissors {}.make_move(&history), Move::Scissors);
        }
    }

    #[test]
    fn round_records_fields() {
        let round = &sample_history()[0];
        assert_eq!(round.my_move, Move::Rock);
        assert_eq!(round.their_move, Move::Paper);
        assert_eq!(round.outcome, Outcome::Loss);
    }
}
