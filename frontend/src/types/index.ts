export type Color = 'white' | 'black';

export type BlunderSeverity = 'inaccuracy' | 'mistake' | 'blunder';

export interface User {
  id: number;
  username: string;
  platform: string;
  last_synced_at: string | null;
  created_at: string;
}

export interface Puzzle {
  id: number;
  game_id: number;
  user_id: number;
  initial_fen: string;
  move_number: number;
  player_color: Color;
  blunder_move_san: string;
  blunder_move_uci: string;
  best_move_san: string;
  best_move_uci: string;
  eval_before: number;
  eval_after_blunder: number;
  eval_after_best: number;
  continuation_uci: string; // JSON array of string
  blunder_continuation_uci: string; // JSON array of string
  tactical_tags: string; // JSON array of string
  blunder_severity: BlunderSeverity;
  opening_name: string | null;
  created_at: string;
}

export interface PuzzleReview {
  id: number;
  puzzle_id: number;
  user_id: number;
  easiness_factor: number;
  interval_days: number;
  repetition_number: number;
  last_reviewed_at: string | null;
  next_due_at: string;
  times_solved: number;
  times_failed: number;
}

export interface PuzzleWithReview {
  puzzle: Puzzle;
  review: PuzzleReview;
  game_white: string;
  game_black: string;
  game_time_class: string;
  game_played_at: string;
}

export interface SyncRequest {
  username: string;
  time_classes?: string[];
  max_games?: number;
  months_back?: number;
  engine_depth?: number;
}

export interface SyncStatus {
  username: string;
  state: 'idle' | 'fetching_games' | 'analyzing' | 'completed' | 'failed';
  total_games: number;
  processed_games: number;
  puzzles_found: number;
  current_game: string | null;
  error: string | null;
}

export interface SolveAttemptRequest {
  user_id: number;
  success: boolean;
  hints_used: number;
  time_taken_ms: number;
  quality?: number;
}

export interface SolveResponse {
  puzzle_id: number;
  success: boolean;
  new_easiness_factor: number;
  new_interval_days: number;
  new_repetition_number: number;
  next_due_at: string;
  is_mastered: boolean;
}

export interface MoveEval {
  uci: string;
  san?: string;
  score_cp: number | null;
  mate_in: number | null;
  pv: string[];
}

export interface EngineEvalResponse {
  fen: string;
  depth: number;
  best_move: string;
  score_cp: number | null;
  mate_in: number | null;
  lines: MoveEval[];
}

export interface StatsSummary {
  total_puzzles: number;
  due_today: number;
  mastered_puzzles: number;
  total_reviews: number;
  retention_rate: number;
  blunders_count: number;
  mistakes_count: number;
  inaccuracies_count: number;
  tactical_tag_breakdown: {
    tag: string;
    count: number;
    success_rate: number;
  }[];
  top_blundered_openings: {
    opening_name: string;
    blunder_count: number;
  }[];
}
