use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// The result of a single round from one player's perspective.
pub enum Outcome {
    Win,
    Loss,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// A rock-paper-scissors move.
pub enum Move {
    Rock,
    Paper,
    Scissors,
}
impl Move {
    /// Returns `Some(true)` if `self` beats `other`, `Some(false)` if it loses,
    /// and `None` when both moves are equal (a draw).
    pub fn beats(&self, other: &Move) -> Option<bool> {
        if self == other {
            None
        } else {
            Some(matches!(
                (self, other),
                (Move::Rock, Move::Scissors)
                    | (Move::Scissors, Move::Paper)
                    | (Move::Paper, Move::Rock)
            ))
        }
    }
}
