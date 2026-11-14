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

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Move; 3] = [Move::Rock, Move::Paper, Move::Scissors];

    #[test]
    fn same_move_is_a_draw() {
        for m in ALL {
            assert_eq!(m.beats(&m), None);
        }
    }

    #[test]
    fn winning_pairs() {
        assert_eq!(Move::Rock.beats(&Move::Scissors), Some(true));
        assert_eq!(Move::Scissors.beats(&Move::Paper), Some(true));
        assert_eq!(Move::Paper.beats(&Move::Rock), Some(true));
    }

    #[test]
    fn losing_pairs() {
        assert_eq!(Move::Scissors.beats(&Move::Rock), Some(false));
        assert_eq!(Move::Paper.beats(&Move::Scissors), Some(false));
        assert_eq!(Move::Rock.beats(&Move::Paper), Some(false));
    }

    #[test]
    fn beats_is_antisymmetric() {
        for a in ALL {
            for b in ALL {
                match (a.beats(&b), b.beats(&a)) {
                    (None, None) => assert_eq!(a, b),
                    (Some(x), Some(y)) => assert_ne!(x, y),
                    other => panic!("inconsistent result for {a:?} vs {b:?}: {other:?}"),
                }
            }
        }
    }
}
