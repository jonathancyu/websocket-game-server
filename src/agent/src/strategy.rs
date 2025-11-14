use common::model::game::{Move, Outcome};
use rand::Rng;

pub struct Round {
    pub my_move: Move,
    pub their_move: Move,
    pub outcome: Outcome,
}

pub trait Strategy {
    fn make_move(&self, history: &Vec<Round>) -> Move;
    fn name(&self) -> &'static str;
}

// Trivial strategies
pub struct OnlyRock {}
impl Strategy for OnlyRock {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Rock
    }
    fn name(&self) -> &'static str {
        "OnlyRock"
    }
}

pub struct OnlyPaper {}
impl Strategy for OnlyPaper {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Paper
    }
    fn name(&self) -> &'static str {
        "OnlyPaper"
    }
}

pub struct OnlyScissors {}
impl Strategy for OnlyScissors {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        Move::Scissors
    }
    fn name(&self) -> &'static str {
        "OnlyScissors"
    }
}

// Random strategy
pub struct RandomMove {}
impl Strategy for RandomMove {
    fn make_move(&self, _: &Vec<Round>) -> Move {
        let mut rng = rand::thread_rng();
        match rng.gen_range(0..3) {
            0 => Move::Rock,
            1 => Move::Paper,
            _ => Move::Scissors,
        }
    }
    fn name(&self) -> &'static str {
        "Random"
    }
}

// Copy opponent's last move
pub struct CopyLastMove {}
impl Strategy for CopyLastMove {
    fn make_move(&self, history: &Vec<Round>) -> Move {
        history
            .last()
            .map(|r| r.their_move.clone())
            .unwrap_or(Move::Rock)
    }
    fn name(&self) -> &'static str {
        "CopyLastMove"
    }
}

// Beat opponent's last move
pub struct BeatLastMove {}
impl Strategy for BeatLastMove {
    fn make_move(&self, history: &Vec<Round>) -> Move {
        let last_move = history
            .last()
            .map(|r| r.their_move.clone())
            .unwrap_or(Move::Rock);

        match last_move {
            Move::Rock => Move::Paper,      // Paper beats Rock
            Move::Paper => Move::Scissors,  // Scissors beats Paper
            Move::Scissors => Move::Rock,   // Rock beats Scissors
        }
    }
    fn name(&self) -> &'static str {
        "BeatLastMove"
    }
}

// Play what beats the most common opponent move
pub struct FrequencyCounter {}
impl Strategy for FrequencyCounter {
    fn make_move(&self, history: &Vec<Round>) -> Move {
        if history.is_empty() {
            return Move::Rock;
        }

        let mut rock_count = 0;
        let mut paper_count = 0;
        let mut scissors_count = 0;

        for round in history {
            match round.their_move {
                Move::Rock => rock_count += 1,
                Move::Paper => paper_count += 1,
                Move::Scissors => scissors_count += 1,
            }
        }

        // Play what beats the most common move
        if rock_count >= paper_count && rock_count >= scissors_count {
            Move::Paper // Beat Rock
        } else if paper_count >= scissors_count {
            Move::Scissors // Beat Paper
        } else {
            Move::Rock // Beat Scissors
        }
    }
    fn name(&self) -> &'static str {
        "FrequencyCounter"
    }
}

// Rotate through moves: Rock -> Paper -> Scissors -> Rock...
pub struct Rotate {}
impl Strategy for Rotate {
    fn make_move(&self, history: &Vec<Round>) -> Move {
        match history.len() % 3 {
            0 => Move::Rock,
            1 => Move::Paper,
            _ => Move::Scissors,
        }
    }
    fn name(&self) -> &'static str {
        "Rotate"
    }
}
