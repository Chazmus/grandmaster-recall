use anyhow::{anyhow, Result};
use regex::Regex;
use shakmaty::fen::Fen;
use shakmaty::san::San;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, Color, Move, Position};
use tracing::{debug, info, warn};

use crate::engine::EnginePool;

#[derive(Debug, Clone)]
pub struct DetectedPuzzle {
    pub initial_fen: String,
    pub move_number: i32,
    pub player_color: String,
    pub blunder_move_san: String,
    pub blunder_move_uci: String,
    pub best_move_san: String,
    pub best_move_uci: String,
    pub eval_before: i32,
    pub eval_after_blunder: i32,
    pub eval_after_best: i32,
    pub continuation_uci: Vec<String>,
    pub blunder_continuation_uci: Vec<String>,
    pub tactical_tags: Vec<String>,
    pub blunder_severity: String,
    pub opening_name: Option<String>,
}

pub struct GameAnalyzer {
    engine: EnginePool,
}

impl GameAnalyzer {
    pub fn new(engine: EnginePool) -> Self {
        Self { engine }
    }

    pub fn extract_opening(pgn: &str) -> Option<String> {
        lazy_static::lazy_static! {
            static ref ECO_URL_RE: Regex = Regex::new(r#"\[ECOUrl\s+"https://www.chess.com/openings/([^"]+)""#).unwrap();
            static ref OPENING_RE: Regex = Regex::new(r#"\[Opening\s+"([^"]+)"\]"#).unwrap();
        }

        if let Some(caps) = ECO_URL_RE.captures(pgn) {
            let slug = caps.get(1).map(|m| m.as_str().replace('-', " "))?;
            return Some(slug);
        }
        if let Some(caps) = OPENING_RE.captures(pgn) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    pub fn parse_pgn_moves(pgn: &str) -> Vec<String> {
        let lines: Vec<&str> = pgn.lines().collect();
        let mut move_text = String::new();

        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                continue;
            }
            if !trimmed.is_empty() {
                move_text.push(' ');
                move_text.push_str(trimmed);
            }
        }

        lazy_static::lazy_static! {
            static ref COMMENT_RE: Regex = Regex::new(r"\{[^}]*\}").unwrap();
            static ref MOVE_NUM_RE: Regex = Regex::new(r"\b\d+\.+").unwrap();
        }

        let no_comments = COMMENT_RE.replace_all(&move_text, " ");
        let no_numbers = MOVE_NUM_RE.replace_all(&no_comments, " ");

        let mut san_moves = Vec::new();
        for token in no_numbers.split_whitespace() {
            let cleaned = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '+' && c != '#' && c != '=' && c != '-');
            if cleaned == "1-0" || cleaned == "0-1" || cleaned == "1/2-1/2" || cleaned == "*" || cleaned.is_empty() {
                continue;
            }
            san_moves.push(cleaned.to_string());
        }

        san_moves
    }

    pub async fn analyze_game_blunders(
        &self,
        pgn: &str,
        target_username: &str,
        white_player: &str,
        black_player: &str,
        eval_depth: u32,
    ) -> Result<Vec<DetectedPuzzle>> {
        self.analyze_game_blunders_throttled(
            pgn,
            target_username,
            white_player,
            black_player,
            eval_depth,
            0,
        )
        .await
    }

    pub async fn analyze_game_blunders_throttled(
        &self,
        pgn: &str,
        target_username: &str,
        white_player: &str,
        black_player: &str,
        eval_depth: u32,
        sleep_ms: u64,
    ) -> Result<Vec<DetectedPuzzle>> {
        let is_white = white_player.to_lowercase() == target_username.to_lowercase();
        let is_black = black_player.to_lowercase() == target_username.to_lowercase();

        if !is_white && !is_black {
            return Err(anyhow!("User {} not part of this game", target_username));
        }

        let user_color = if is_white { Color::White } else { Color::Black };
        let opening = Self::extract_opening(pgn);
        let san_tokens = Self::parse_pgn_moves(pgn);

        let mut pos = Chess::default();
        let mut detected_puzzles = Vec::new();

        for (i, san_str) in san_tokens.iter().enumerate() {
            let turn_color = pos.turn();
            let is_user_turn = turn_color == user_color;
            let move_number = (i as i32 / 2) + 1;

            let parsed_san = match san_str.parse::<San>() {
                Ok(s) => s,
                Err(e) => {
                    debug!("Failed to parse SAN '{}': {:?}", san_str, e);
                    break;
                }
            };

            let played_move = match parsed_san.to_move(&pos) {
                Ok(m) => m,
                Err(e) => {
                    debug!("Illegal SAN '{}' in position: {:?}", san_str, e);
                    break;
                }
            };

            let played_uci = played_move.to_uci(CastlingMode::Standard).to_string();
            let fen_before = Fen::from_position(pos.clone(), shakmaty::EnPassantMode::Legal).to_string();

            if is_user_turn && move_number >= 2 {
                // Skip positions with 0 or 1 legal moves (forced moves/check evasions)
                if pos.legal_moves().len() <= 1 {
                    pos.play_unchecked(&played_move);
                    continue;
                }

                if sleep_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                }

                let eval_before_res = match self.engine.evaluate_fen(&fen_before, eval_depth, 2).await {
                    Ok(r) => r,
                    Err(e) => {
                        warn!("Stockfish eval error on FEN {}: {:?}", fen_before, e);
                        pos.play_unchecked(&played_move);
                        continue;
                    }
                };

                let best_move_uci = eval_before_res.best_move.clone();

                if !best_move_uci.is_empty() && best_move_uci != played_uci {
                    let mut pos_after = pos.clone();
                    pos_after.play_unchecked(&played_move);

                    // Delivering checkmate can NEVER be a blunder or mistake!
                    if pos_after.is_checkmate() {
                        pos.play_unchecked(&played_move);
                        continue;
                    }

                    let fen_after = Fen::from_position(pos_after.clone(), shakmaty::EnPassantMode::Legal).to_string();

                    if sleep_ms > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms / 2)).await;
                    }

                    let eval_after_res = match self.engine.evaluate_fen(&fen_after, eval_depth, 1).await {
                        Ok(r) => r,
                        Err(_) => {
                            pos.play_unchecked(&played_move);
                            continue;
                        }
                    };

                    let eval_before_pov = Self::score_from_side_to_move(
                        eval_before_res.score_cp,
                        eval_before_res.mate_in,
                    );

                    let eval_after_pov = if pos_after.is_checkmate() {
                        10000
                    } else if pos_after.is_stalemate() || pos_after.is_insufficient_material() {
                        0
                    } else {
                        -Self::score_from_side_to_move(
                            eval_after_res.score_cp,
                            eval_after_res.mate_in,
                        )
                    };

                    let eval_drop = eval_before_pov - eval_after_pov;

                    let severity = if eval_drop >= 250 || (eval_before_pov > -150 && eval_after_pov < -400) {
                        "blunder"
                    } else if eval_drop >= 120 {
                        "mistake"
                    } else if eval_drop >= 60 {
                        "inaccuracy"
                    } else {
                        ""
                    };

                    if !severity.is_empty() {
                        let best_san = if let Ok(uci_parsed) = best_move_uci.parse::<UciMove>() {
                            if let Ok(m) = uci_parsed.to_move(&pos) {
                                San::from_move(&pos, &m).to_string()
                            } else {
                                best_move_uci.clone()
                            }
                        } else {
                            best_move_uci.clone()
                        };

                        // Limit continuation to at most 3 plies (e.g. Player move -> Opponent reply -> Player win)
                        let continuation_uci = if let Some(first_line) = eval_before_res.lines.first() {
                            first_line.pv.iter().take(3).cloned().collect()
                        } else {
                            vec![best_move_uci.clone()]
                        };

                        // Limit blunder punishment demonstration to 2 plies
                        let blunder_continuation_uci = if let Some(first_line) = eval_after_res.lines.first() {
                            first_line.pv.iter().take(2).cloned().collect()
                        } else {
                            Vec::new()
                        };

                        let tactical_tags = Self::detect_tactical_tags(
                            &pos,
                            &played_move,
                            &best_move_uci,
                            move_number,
                            severity,
                        );

                        detected_puzzles.push(DetectedPuzzle {
                            initial_fen: fen_before,
                            move_number,
                            player_color: if user_color == Color::White { "white".to_string() } else { "black".to_string() },
                            blunder_move_san: san_str.clone(),
                            blunder_move_uci: played_uci,
                            best_move_san: best_san,
                            best_move_uci,
                            eval_before: eval_before_pov,
                            eval_after_blunder: eval_after_pov,
                            eval_after_best: eval_before_pov,
                            continuation_uci,
                            blunder_continuation_uci,
                            tactical_tags,
                            blunder_severity: severity.to_string(),
                            opening_name: opening.clone(),
                        });
                    }
                }
            }

            pos.play_unchecked(&played_move);
        }

        info!("Extracted {} blunder/mistake puzzles from game", detected_puzzles.len());
        Ok(detected_puzzles)
    }


    pub fn score_from_side_to_move(score_cp: Option<i32>, mate_in: Option<i32>) -> i32 {
        if let Some(m) = mate_in {
            if m > 0 {
                10000 - (m.abs() * 100)
            } else if m < 0 {
                -10000 + (m.abs() * 100)
            } else {
                -10000
            }
        } else {
            score_cp.unwrap_or(0)
        }
    }

    pub fn score_from_pov(score_cp: Option<i32>, mate_in: Option<i32>, is_active_player: bool) -> i32 {
        let raw = Self::score_from_side_to_move(score_cp, mate_in);
        if is_active_player {
            raw
        } else {
            -raw
        }
    }

    pub fn detect_tactical_tags(
        pos: &Chess,
        played_move: &Move,
        best_uci: &str,
        move_number: i32,
        severity: &str,
    ) -> Vec<String> {
        let mut tags = Vec::new();

        if move_number <= 10 {
            tags.push("Opening Mistake".to_string());
        }

        let board = pos.board();
        let total_pieces = board.white().count() + board.black().count();
        if total_pieces <= 10 {
            tags.push("Endgame Blunder".to_string());
        }

        if let Ok(best_uci_parsed) = best_uci.parse::<UciMove>() {
            if let Ok(best_m) = best_uci_parsed.to_move(pos) {
                if best_m.is_capture() {
                    tags.push("Hanging Piece".to_string());
                }
            }
        }

        if played_move.is_capture() {
            tags.push("Tactical Trade".to_string());
        }

        if severity == "blunder" {
            tags.push("Missed Tactic".to_string());
        }

        let turn = pos.turn();
        let opponent_king_sq = pos.board().king_of(!turn);
        if let Some(sq) = opponent_king_sq {
            let rank = sq.rank();
            if (turn == Color::White && rank == shakmaty::Rank::Eighth)
                || (turn == Color::Black && rank == shakmaty::Rank::First)
            {
                tags.push("Back Rank".to_string());
            }
        }

        if tags.is_empty() {
            tags.push("Positional Blunder".to_string());
        }

        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_opening() {
        let pgn1 = r#"[Event "Live Chess"]
[Site "Chess.com"]
[ECOUrl "https://www.chess.com/openings/Sicilian-Defense-Najdorf-Variation"]
1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6"#;
        assert_eq!(
            GameAnalyzer::extract_opening(pgn1),
            Some("Sicilian Defense Najdorf Variation".to_string())
        );

        let pgn2 = r#"[Event "Live Chess"]
[Opening "Queen's Gambit Declined: Traditional"]
1. d4 d5 2. c4 e6"#;
        assert_eq!(
            GameAnalyzer::extract_opening(pgn2),
            Some("Queen's Gambit Declined: Traditional".to_string())
        );
    }

    #[test]
    fn test_parse_pgn_moves() {
        let pgn = r#"[Event "Live Chess"]
[White "Player1"]
[Black "Player2"]

1. e4 {[%clk 0:03:00]} 1... e5 {[%clk 0:02:58]} 2. Nf3 {[%clk 0:02:55]} 2... Nc6 {[%clk 0:02:50]} 3. Bb5 1-0"#;

        let moves = GameAnalyzer::parse_pgn_moves(pgn);
        assert_eq!(moves, vec!["e4", "e5", "Nf3", "Nc6", "Bb5"]);
    }

    #[test]
    fn test_score_from_side_to_move() {
        assert_eq!(GameAnalyzer::score_from_side_to_move(Some(150), None), 150);
        assert_eq!(GameAnalyzer::score_from_side_to_move(None, Some(1)), 9900);
        assert_eq!(GameAnalyzer::score_from_side_to_move(None, Some(-1)), -9900);
        assert_eq!(GameAnalyzer::score_from_side_to_move(None, Some(0)), -10000);
    }

    #[test]
    fn test_score_from_pov() {
        assert_eq!(GameAnalyzer::score_from_pov(Some(150), None, true), 150);
        assert_eq!(GameAnalyzer::score_from_pov(Some(150), None, false), -150);
        assert_eq!(GameAnalyzer::score_from_pov(None, Some(2), true), 9800);
        assert_eq!(GameAnalyzer::score_from_pov(None, Some(2), false), -9800);
    }

    #[test]
    fn test_tactical_tags_detection() {
        let pos = Chess::default();
        let move_parsed = "e2e4".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &move_parsed, "d2d4", 2, "blunder");
        assert!(tags.contains(&"Opening Mistake".to_string()));
        assert!(tags.contains(&"Missed Tactic".to_string()));
    }
}
