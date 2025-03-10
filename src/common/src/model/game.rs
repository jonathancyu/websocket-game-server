use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Outcome {
    Win,
    Loss,
    Draw,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Move {
    Rock,
    Paper,
    Scissors,
}
impl Move {
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
