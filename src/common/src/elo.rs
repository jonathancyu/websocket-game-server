/// ELO rating system implementation
///
/// Uses the standard ELO formula:
/// new_rating = old_rating + K * (actual_score - expected_score)
///
/// Where:
/// - K is the K-factor (default 32)
/// - expected_score = 1 / (1 + 10^((opponent_rating - player_rating) / 400))
/// - actual_score is 1 for win, 0.5 for draw, 0 for loss

const DEFAULT_K_FACTOR: f64 = 32.0;
const DEFAULT_RATING: i32 = 1500;

/// Calculate the expected score for a player given their rating and their opponent's rating
pub fn expected_score(player_rating: i32, opponent_rating: i32) -> f64 {
    1.0 / (1.0 + 10_f64.powf((opponent_rating - player_rating) as f64 / 400.0))
}

/// Calculate new ELO rating for a player after a match
///
/// # Arguments
/// * `current_rating` - The player's current ELO rating
/// * `opponent_rating` - The opponent's ELO rating
/// * `actual_score` - The actual score: 1.0 for win, 0.5 for draw, 0.0 for loss
/// * `k_factor` - The K-factor (optional, defaults to 32.0)
///
/// # Returns
/// The new ELO rating (rounded to nearest integer)
pub fn calculate_new_rating(
    current_rating: i32,
    opponent_rating: i32,
    actual_score: f64,
    k_factor: Option<f64>,
) -> i32 {
    let k = k_factor.unwrap_or(DEFAULT_K_FACTOR);
    let expected = expected_score(current_rating, opponent_rating);
    let rating_change = k * (actual_score - expected);
    (current_rating as f64 + rating_change).round() as i32
}

/// Get the default starting ELO rating
pub fn default_rating() -> i32 {
    DEFAULT_RATING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_score_equal_ratings() {
        let score = expected_score(1500, 1500);
        assert!((score - 0.5).abs() < 0.01, "Equal ratings should give 0.5 expected score");
    }

    #[test]
    fn test_expected_score_higher_rating() {
        let score = expected_score(1600, 1500);
        assert!(score > 0.5, "Higher rating should have > 0.5 expected score");
    }

    #[test]
    fn test_expected_score_lower_rating() {
        let score = expected_score(1400, 1500);
        assert!(score < 0.5, "Lower rating should have < 0.5 expected score");
    }

    #[test]
    fn test_calculate_new_rating_win() {
        let new_rating = calculate_new_rating(1500, 1500, 1.0, None);
        assert!(new_rating > 1500, "Win should increase rating");
    }

    #[test]
    fn test_calculate_new_rating_loss() {
        let new_rating = calculate_new_rating(1500, 1500, 0.0, None);
        assert!(new_rating < 1500, "Loss should decrease rating");
    }

    #[test]
    fn test_calculate_new_rating_draw() {
        let new_rating = calculate_new_rating(1500, 1500, 0.5, None);
        assert_eq!(new_rating, 1500, "Draw between equal ratings should not change rating");
    }
}

