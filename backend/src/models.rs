use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub platform: String,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Game {
    pub id: i64,
    pub user_id: i64,
    pub chess_com_id: String,
    pub time_class: String,
    pub white_player: String,
    pub black_player: String,
    pub result: String,
    pub user_color: String, // "white" or "black"
    pub pgn: String,
    pub played_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Puzzle {
    pub id: i64,
    pub game_id: i64,
    pub user_id: i64,
    pub initial_fen: String,
    pub move_number: i32,
    pub player_color: String, // "white" or "black"
    pub blunder_move_san: String,
    pub blunder_move_uci: String,
    pub best_move_san: String,
    pub best_move_uci: String,
    pub eval_before: i32,        // Centipawns from player's POV
    pub eval_after_blunder: i32, // Centipawns from player's POV
    pub eval_after_best: i32,    // Centipawns from player's POV
    pub continuation_uci: String, // JSON array of UCI strings
    pub blunder_continuation_uci: String, // Why player move was bad (punishing response)
    pub tactical_tags: String,   // JSON array of strings
    pub blunder_severity: String, // "inaccuracy", "mistake", "blunder"
    pub opening_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PuzzleReview {
    pub id: i64,
    pub puzzle_id: i64,
    pub user_id: i64,
    pub easiness_factor: f64,
    pub interval_days: i32,
    pub repetition_number: i32,
    pub last_reviewed_at: Option<DateTime<Utc>>,
    pub next_due_at: DateTime<Utc>,
    pub times_solved: i32,
    pub times_failed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleWithReview {
    pub puzzle: Puzzle,
    pub review: PuzzleReview,
    pub game_white: String,
    pub game_black: String,
    pub game_time_class: String,
    pub game_played_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequest {
    pub username: String,
    pub time_classes: Option<Vec<String>>, // default: ["rapid", "blitz"]
    pub max_games: Option<usize>,         // default: 30
    pub months_back: Option<usize>,       // default: 2
    pub engine_depth: Option<u32>,        // default: 14
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub username: String,
    pub state: String, // "idle", "fetching_games", "analyzing", "completed", "failed"
    pub total_games: usize,
    pub processed_games: usize,
    pub puzzles_found: usize,
    pub current_game: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolveAttemptRequest {
    pub user_id: i64,
    pub success: bool,
    pub hints_used: i32,
    pub time_taken_ms: i64,
    pub quality: Option<u8>, // Optional 0-5 direct rating from user
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResponse {
    pub puzzle_id: i64,
    pub success: bool,
    pub new_easiness_factor: f64,
    pub new_interval_days: i32,
    pub new_repetition_number: i32,
    pub next_due_at: DateTime<Utc>,
    pub is_mastered: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EngineEvalRequest {
    pub fen: String,
    pub depth: Option<u32>,
    pub multi_pv: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveEval {
    pub uci: String,
    pub san: Option<String>,
    pub score_cp: Option<i32>,
    pub mate_in: Option<i32>,
    pub pv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineEvalResponse {
    pub fen: String,
    pub depth: u32,
    pub best_move: String,
    pub score_cp: Option<i32>,
    pub mate_in: Option<i32>,
    pub lines: Vec<MoveEval>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidateMoveRequest {
    pub fen: String,
    pub move_uci: String,
    pub expected_best_uci: String,
    pub player_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateMoveResponse {
    pub is_valid: bool,
    pub is_best: bool,
    pub eval_diff_cp: i32,
    pub explanation: String,
    pub opponent_reply_uci: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub total_puzzles: i64,
    pub due_today: i64,
    pub mastered_puzzles: i64,
    pub total_reviews: i64,
    pub retention_rate: f64,
    pub blunders_count: i64,
    pub mistakes_count: i64,
    pub inaccuracies_count: i64,
    pub tactical_tag_breakdown: Vec<TacticalTagStat>,
    pub top_blundered_openings: Vec<OpeningStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalTagStat {
    pub tag: String,
    pub count: i64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningStat {
    pub opening_name: String,
    pub blunder_count: i64,
}
