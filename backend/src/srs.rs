use chrono::{DateTime, Duration, Utc};

#[derive(Debug, PartialEq)]
pub struct SrsResult {
    pub new_ef: f64,
    pub new_interval_days: i32,
    pub new_repetition_number: i32,
    pub next_due_at: DateTime<Utc>,
    pub is_mastered: bool,
}

pub struct Sm2Engine;

impl Sm2Engine {
    pub fn calculate_quality(success: bool, hints_used: i32, time_taken_ms: i64) -> u8 {
        if !success {
            if hints_used >= 2 {
                return 0; // Failed even with multiple hints
            } else {
                return 1; // Failed
            }
        }

        // Success
        if hints_used == 0 {
            if time_taken_ms < 6000 {
                5 // Instant tactical pattern recognition
            } else if time_taken_ms < 20000 {
                4 // Solved reasonably quickly
            } else {
                3 // Solved with effort/calculation
            }
        } else if hints_used == 1 {
            3 // Needed 1 minor hint (e.g. piece square)
        } else {
            2 // Needed multiple hints
        }
    }

    pub fn calculate_next_schedule(
        current_ef: f64,
        current_interval_days: i32,
        current_reps: i32,
        quality: u8,
    ) -> SrsResult {
        let q = quality as f64;

        // Calculate new Easiness Factor (EF)
        // EF' = EF + (0.1 - (5 - q) * (0.08 + (5 - q) * 0.02))
        let delta_ef = 0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02);
        let mut new_ef = current_ef + delta_ef;
        if new_ef < 1.3 {
            new_ef = 1.3;
        }

        let (new_reps, new_interval) = if quality < 3 {
            // Failed: reset repetition streak, review in 1 day
            (0, 1)
        } else {
            // Succeeded: advance interval according to SM-2 formula
            match current_reps {
                0 => (1, 1),
                1 => (2, 3),
                _ => {
                    let next_days = ((current_interval_days as f64) * new_ef).round() as i32;
                    let safe_days = next_days.max(current_interval_days + 1);
                    (current_reps + 1, safe_days)
                }
            }
        };

        let next_due_at = Utc::now() + Duration::days(new_interval as i64);
        let is_mastered = new_interval >= 21;

        SrsResult {
            new_ef,
            new_interval_days: new_interval,
            new_repetition_number: new_reps,
            next_due_at,
            is_mastered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_quality() {
        // Fast solve without hints -> 5
        assert_eq!(Sm2Engine::calculate_quality(true, 0, 3500), 5);
        // Moderate solve without hints -> 4
        assert_eq!(Sm2Engine::calculate_quality(true, 0, 12000), 4);
        // Slow solve without hints -> 3
        assert_eq!(Sm2Engine::calculate_quality(true, 0, 25000), 3);
        // Solved with 1 hint -> 3
        assert_eq!(Sm2Engine::calculate_quality(true, 1, 4000), 3);
        // Solved with 2 hints -> 2
        assert_eq!(Sm2Engine::calculate_quality(true, 2, 4000), 2);
        // Failed without hints -> 1
        assert_eq!(Sm2Engine::calculate_quality(false, 0, 10000), 1);
        // Failed with multiple hints -> 0
        assert_eq!(Sm2Engine::calculate_quality(false, 2, 10000), 0);
    }

    #[test]
    fn test_sm2_first_successful_review() {
        // Brand new puzzle (reps = 0, interval = 0, ef = 2.5), rated perfect (5)
        let res = Sm2Engine::calculate_next_schedule(2.5, 0, 0, 5);
        assert_eq!(res.new_repetition_number, 1);
        assert_eq!(res.new_interval_days, 1);
        assert!((res.new_ef - 2.6).abs() < 0.001);
        assert!(!res.is_mastered);
    }

    #[test]
    fn test_sm2_second_successful_review() {
        // Rep 1 puzzle -> interval should advance to 3
        let res = Sm2Engine::calculate_next_schedule(2.6, 1, 1, 4);
        assert_eq!(res.new_repetition_number, 2);
        assert_eq!(res.new_interval_days, 3);
        assert!(!res.is_mastered);
    }

    #[test]
    fn test_sm2_subsequent_reviews_exponential_growth() {
        // Rep 2 puzzle, interval 3, ef 2.5 -> next interval = round(3 * 2.5) = 8
        let res = Sm2Engine::calculate_next_schedule(2.5, 3, 2, 5);
        assert_eq!(res.new_repetition_number, 3);
        assert_eq!(res.new_interval_days, 8);
        assert!(!res.is_mastered);

        // Rep 3 puzzle, interval 8, ef 2.6 -> next interval = round(8 * 2.6) = 21 (Mastered!)
        let res2 = Sm2Engine::calculate_next_schedule(res.new_ef, res.new_interval_days, res.new_repetition_number, 5);
        assert_eq!(res2.new_repetition_number, 4);
        assert_eq!(res2.new_interval_days, 22);
        assert!(res2.is_mastered);
    }

    #[test]
    fn test_sm2_failure_resets_streak() {
        // Failure on previously mastered puzzle (quality 1)
        let res = Sm2Engine::calculate_next_schedule(2.5, 21, 5, 1);
        assert_eq!(res.new_repetition_number, 0);
        assert_eq!(res.new_interval_days, 1);
        assert!(!res.is_mastered);
        assert!(res.new_ef < 2.5);
    }

    #[test]
    fn test_sm2_ef_lower_bound() {
        // Repeated low ratings should not lower EF below 1.3
        let mut ef = 1.4;
        for _ in 0..5 {
            let res = Sm2Engine::calculate_next_schedule(ef, 1, 0, 0);
            ef = res.new_ef;
        }
        assert_eq!(ef, 1.3);
    }
}
