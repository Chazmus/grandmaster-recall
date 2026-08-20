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

        // Reset game state once at the start of the game analysis to clean TT cache
        let _ = self.engine.reset_game().await;

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

                let eval_before_res = match self.engine.evaluate_fen(&fen_before, eval_depth, 1).await {
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

    pub fn piece_value(role: shakmaty::Role) -> i32 {
        match role {
            shakmaty::Role::Pawn => 100,
            shakmaty::Role::Knight => 300,
            shakmaty::Role::Bishop => 300,
            shakmaty::Role::Rook => 500,
            shakmaty::Role::Queen => 900,
            shakmaty::Role::King => 10000,
        }
    }

    pub fn is_defended_by(board: &shakmaty::Board, sq: shakmaty::Square, color: shakmaty::Color) -> bool {
        let occupied = board.occupied();
        !board.attacks_to(sq, color, occupied).is_empty()
    }

    pub fn slider_attacks(role: shakmaty::Role, sq: shakmaty::Square, occupied: shakmaty::Bitboard) -> shakmaty::Bitboard {
        match role {
            shakmaty::Role::Bishop => shakmaty::attacks::bishop_attacks(sq, occupied),
            shakmaty::Role::Rook => shakmaty::attacks::rook_attacks(sq, occupied),
            shakmaty::Role::Queen => shakmaty::attacks::queen_attacks(sq, occupied),
            _ => shakmaty::Bitboard::EMPTY,
        }
    }

    pub fn piece_attacks(role: shakmaty::Role, color: shakmaty::Color, sq: shakmaty::Square, occupied: shakmaty::Bitboard) -> shakmaty::Bitboard {
        match role {
            shakmaty::Role::Pawn => shakmaty::attacks::pawn_attacks(color, sq),
            shakmaty::Role::Knight => shakmaty::attacks::knight_attacks(sq),
            shakmaty::Role::Bishop => shakmaty::attacks::bishop_attacks(sq, occupied),
            shakmaty::Role::Rook => shakmaty::attacks::rook_attacks(sq, occupied),
            shakmaty::Role::Queen => shakmaty::attacks::queen_attacks(sq, occupied),
            shakmaty::Role::King => shakmaty::attacks::king_attacks(sq),
        }
    }

    pub fn detect_fork(board_after: &shakmaty::Board, best_m: &Move, turn: Color) -> bool {
        let to_sq = best_m.to();
        let role = best_m.role();
        let occupied = board_after.occupied();
        let attacks = Self::piece_attacks(role, turn, to_sq, occupied) & board_after.by_color(!turn);

        let mut valuable_targets = 0;
        for target_sq in attacks {
            if let Some(target_role) = board_after.role_at(target_sq) {
                let is_king = target_role == shakmaty::Role::King;
                let is_higher_val = Self::piece_value(target_role) >= Self::piece_value(role);
                let is_undefended = !Self::is_defended_by(board_after, target_sq, !turn);
                if is_king || is_higher_val || is_undefended {
                    valuable_targets += 1;
                }
            }
        }
        valuable_targets >= 2
    }

    pub fn detect_pin(board_after: &shakmaty::Board, best_m: &Move, turn: Color) -> bool {
        let occupied = board_after.occupied();
        let to_sq = best_m.to();
        let opp_color = !turn;
        let opp_pieces = board_after.by_color(opp_color);

        let friendly_sliders = (board_after.bishops_and_queens() | board_after.rooks_and_queens()) & board_after.by_color(turn);

        for slider_sq in friendly_sliders {
            let slider_role = board_after.role_at(slider_sq).unwrap();
            for target_sq in opp_pieces {
                if target_sq == slider_sq {
                    continue;
                }
                let target_role = board_after.role_at(target_sq).unwrap();
                let is_high_target = target_role == shakmaty::Role::King
                    || target_role == shakmaty::Role::Queen
                    || (slider_role == shakmaty::Role::Bishop && target_role == shakmaty::Role::Rook);
                if !is_high_target {
                    continue;
                }

                let base_attacks = Self::slider_attacks(slider_role, slider_sq, shakmaty::Bitboard::EMPTY);
                if base_attacks.contains(target_sq) {
                    let between = shakmaty::attacks::between(slider_sq, target_sq) & occupied;
                    if between.count() == 1 {
                        let pinned_sq = between.first().unwrap();
                        if board_after.color_at(pinned_sq) == Some(opp_color) {
                            let pinned_role = board_after.role_at(pinned_sq).unwrap();
                            if target_role == shakmaty::Role::King || Self::piece_value(target_role) > Self::piece_value(pinned_role) {
                                if slider_sq == to_sq
                                    || to_sq == pinned_sq
                                    || Self::piece_attacks(best_m.role(), turn, to_sq, occupied).contains(pinned_sq)
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn detect_skewer(board_after: &shakmaty::Board, best_m: &Move, turn: Color) -> bool {
        let occupied = board_after.occupied();
        let to_sq = best_m.to();
        let opp_color = !turn;
        let opp_pieces = board_after.by_color(opp_color);

        let friendly_sliders = (board_after.bishops_and_queens() | board_after.rooks_and_queens()) & board_after.by_color(turn);

        for slider_sq in friendly_sliders {
            let slider_role = board_after.role_at(slider_sq).unwrap();
            let direct_attacks = Self::slider_attacks(slider_role, slider_sq, occupied) & opp_pieces;
            for front_sq in direct_attacks {
                let front_role = board_after.role_at(front_sq).unwrap();
                let ray = shakmaty::attacks::ray(slider_sq, front_sq);

                for back_sq in opp_pieces {
                    if back_sq == front_sq || back_sq == slider_sq {
                        continue;
                    }
                    if ray.contains(back_sq) {
                        let between_slider_back = shakmaty::attacks::between(slider_sq, back_sq);
                        if between_slider_back.contains(front_sq) {
                            let between_front_back = shakmaty::attacks::between(front_sq, back_sq) & occupied;
                            if between_front_back.is_empty() {
                                let back_role = board_after.role_at(back_sq).unwrap();
                                if front_role == shakmaty::Role::King || Self::piece_value(front_role) >= Self::piece_value(back_role) {
                                    if slider_sq == to_sq {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn detect_discovered_attack(pos_before: &Chess, pos_after: &Chess, best_m: &Move, turn: Color) -> (bool, bool) {
        let from_sq = match best_m.from() {
            Some(sq) => sq,
            None => return (false, false),
        };
        let to_sq = best_m.to();
        let opp_color = !turn;
        let opp_king = pos_after.board().king_of(opp_color);

        let occupied_before = pos_before.board().occupied();
        let occupied_after = pos_after.board().occupied();

        let friendly_sliders_after = (pos_after.board().bishops_and_queens() | pos_after.board().rooks_and_queens())
            & pos_after.board().by_color(turn);

        let mut disc_attack = false;
        let mut disc_check = false;

        for slider_sq in friendly_sliders_after {
            if slider_sq == to_sq {
                continue;
            }
            let slider_role = pos_after.board().role_at(slider_sq).unwrap();

            let base_attacks = Self::slider_attacks(slider_role, slider_sq, shakmaty::Bitboard::EMPTY);
            if !base_attacks.contains(from_sq) {
                continue;
            }

            let attacks_before = Self::slider_attacks(slider_role, slider_sq, occupied_before);
            let attacks_after = Self::slider_attacks(slider_role, slider_sq, occupied_after);

            let new_attacks = attacks_after & !attacks_before & pos_after.board().by_color(opp_color);

            if let Some(k_sq) = opp_king {
                if new_attacks.contains(k_sq) {
                    disc_check = true;
                }
            }

            if !new_attacks.is_empty() {
                disc_attack = true;
            }
        }

        (disc_attack, disc_check)
    }

    pub fn detect_double_check(pos_after: &Chess) -> bool {
        pos_after.checkers().count() > 1
    }

    pub fn detect_smothered_mate(pos_after: &Chess, best_m: &Move, turn: Color) -> bool {
        if !pos_after.is_checkmate() || best_m.role() != shakmaty::Role::Knight {
            return false;
        }
        let opp_king_sq = match pos_after.board().king_of(!turn) {
            Some(sq) => sq,
            None => return false,
        };
        let adjacent = shakmaty::attacks::king_attacks(opp_king_sq);
        let occupied_by_opp = adjacent & pos_after.board().by_color(!turn);
        (adjacent & !pos_after.board().occupied()).is_empty() && occupied_by_opp.count() >= 3
    }

    pub fn detect_back_rank(pos_after: &Chess, best_m: &Move, turn: Color) -> bool {
        let opp_color = !turn;
        let opp_king_sq = match pos_after.board().king_of(opp_color) {
            Some(sq) => sq,
            None => return false,
        };
        let rank = opp_king_sq.rank();
        let is_back_rank = (opp_color == Color::Black && rank == shakmaty::Rank::Eighth)
            || (opp_color == Color::White && rank == shakmaty::Rank::First);

        if !is_back_rank {
            return false;
        }

        let is_rook_or_queen = best_m.role() == shakmaty::Role::Rook || best_m.role() == shakmaty::Role::Queen;
        let delivers_check = pos_after.is_check();
        let on_back_rank = best_m.to().rank() == rank;

        (pos_after.is_checkmate() && on_back_rank) || (delivers_check && on_back_rank && is_rook_or_queen)
    }

    pub fn detect_hanging_piece(pos_before: &Chess, best_m: &Move, turn: Color) -> bool {
        if !best_m.is_capture() {
            return false;
        }
        let to_sq = best_m.to();
        let opp_color = !turn;
        let captured_role = match pos_before.board().role_at(to_sq) {
            Some(r) => r,
            None => return false,
        };
        let is_undefended = !Self::is_defended_by(pos_before.board(), to_sq, opp_color);
        let is_gain = Self::piece_value(captured_role) > Self::piece_value(best_m.role());
        is_undefended || is_gain
    }

    pub fn detect_trapped_piece(pos_after: &Chess, best_m: &Move, turn: Color) -> bool {
        let to_sq = best_m.to();
        let opp_color = !turn;
        let occupied = pos_after.board().occupied();
        let attacked_opp = Self::piece_attacks(best_m.role(), turn, to_sq, occupied) & pos_after.board().by_color(opp_color);

        for target_sq in attacked_opp {
            let role = pos_after.board().role_at(target_sq).unwrap();
            if role == shakmaty::Role::Pawn || role == shakmaty::Role::King {
                continue;
            }
            let target_attacks = Self::piece_attacks(role, opp_color, target_sq, occupied);
            let escape_squares = target_attacks & !pos_after.board().by_color(opp_color);
            let mut safe_squares = 0;
            for esc_sq in escape_squares {
                if !Self::is_defended_by(pos_after.board(), esc_sq, turn) {
                    safe_squares += 1;
                }
            }
            if safe_squares == 0 {
                return true;
            }
        }
        false
    }

    pub fn detect_passed_pawn_or_promotion(best_m: &Move, turn: Color) -> Option<String> {
        if best_m.promotion().is_some() {
            return Some("Pawn Promotion".to_string());
        }
        if best_m.role() == shakmaty::Role::Pawn {
            let rank = best_m.to().rank();
            if (turn == Color::White && rank >= shakmaty::Rank::Seventh)
                || (turn == Color::Black && rank <= shakmaty::Rank::Second)
            {
                return Some("Advanced Passed Pawn".to_string());
            }
        }
        None
    }

    pub fn generate_explanation(tags: &[String], best_san: Option<&str>) -> String {
        let prefix = match best_san {
            Some(san) => format!("{}: ", san),
            None => "".to_string(),
        };

        if tags.contains(&"Smothered Mate".to_string()) {
            return format!("{}Delivers a smothered checkmate with the knight!", prefix);
        }
        if tags.contains(&"Back Rank".to_string()) {
            return format!("{}Exploits the weak back rank.", prefix);
        }
        if tags.contains(&"Double Check".to_string()) {
            return format!("{}Delivers a powerful double check.", prefix);
        }
        if tags.contains(&"Discovered Check".to_string()) {
            return format!("{}Unleashes a discovered check against the king.", prefix);
        }
        if tags.contains(&"Discovered Attack".to_string()) {
            return format!("{}Unleashes a discovered attack from behind.", prefix);
        }
        if tags.contains(&"Fork".to_string()) {
            return format!("{}Forks multiple enemy pieces simultaneously.", prefix);
        }
        if tags.contains(&"Pin".to_string()) {
            return format!("{}Pins an enemy piece to their king or queen.", prefix);
        }
        if tags.contains(&"Skewer".to_string()) {
            return format!("{}Skewers a high-value piece to win material.", prefix);
        }
        if tags.contains(&"Trapped Piece".to_string()) {
            return format!("{}Traps an enemy piece with no safe escape.", prefix);
        }
        if tags.contains(&"Hanging Piece".to_string()) {
            return format!("{}Wins an undefended hanging piece.", prefix);
        }
        if tags.contains(&"Pawn Promotion".to_string()) {
            return format!("{}Promotes to a new queen.", prefix);
        }
        if tags.contains(&"Advanced Passed Pawn".to_string()) {
            return format!("{}Advances a dangerous passed pawn.", prefix);
        }

        format!("{}Best tactical continuation.", prefix)
    }

    pub fn detect_tactical_tags(
        pos: &Chess,
        played_move: &Move,
        best_uci: &str,
        move_number: i32,
        severity: &str,
    ) -> Vec<String> {
        let mut tags = Vec::new();
        let turn = pos.turn();

        let best_m_opt = best_uci
            .parse::<UciMove>()
            .ok()
            .and_then(|u| u.to_move(pos).ok());

        if let Some(best_m) = best_m_opt {
            let mut pos_after_best = pos.clone();
            pos_after_best.play_unchecked(&best_m);
            let board_after = pos_after_best.board();

            // 1. Checkmate specific patterns
            if Self::detect_smothered_mate(&pos_after_best, &best_m, turn) {
                tags.push("Smothered Mate".to_string());
            }
            if Self::detect_back_rank(&pos_after_best, &best_m, turn) {
                tags.push("Back Rank".to_string());
            }
            if Self::detect_double_check(&pos_after_best) {
                tags.push("Double Check".to_string());
            }

            // 2. Discovered attacks / checks
            let (disc_attack, disc_check) =
                Self::detect_discovered_attack(pos, &pos_after_best, &best_m, turn);
            if disc_check {
                tags.push("Discovered Check".to_string());
            } else if disc_attack {
                tags.push("Discovered Attack".to_string());
            }

            // 3. Forks / Double Attacks
            if Self::detect_fork(board_after, &best_m, turn) {
                tags.push("Fork".to_string());
            }

            // 4. Pins
            if Self::detect_pin(board_after, &best_m, turn) {
                tags.push("Pin".to_string());
            }

            // 5. Skewers
            if Self::detect_skewer(board_after, &best_m, turn) {
                tags.push("Skewer".to_string());
            }

            // 6. Hanging piece
            if Self::detect_hanging_piece(pos, &best_m, turn) {
                tags.push("Hanging Piece".to_string());
            }

            // 7. Trapped piece
            if Self::detect_trapped_piece(&pos_after_best, &best_m, turn) {
                tags.push("Trapped Piece".to_string());
            }

            // 8. Passed pawn / promotion
            if let Some(pawn_tag) = Self::detect_passed_pawn_or_promotion(&best_m, turn) {
                tags.push(pawn_tag);
            }
        }

        // Game phase & context tags
        if move_number <= 10 {
            tags.push("Opening Mistake".to_string());
        }

        let total_pieces = pos.board().white().count() + pos.board().black().count();
        if total_pieces <= 10 {
            tags.push("Endgame Tactic".to_string());
        }

        if played_move.is_capture() {
            tags.push("Tactical Trade".to_string());
        }

        if severity == "blunder" && tags.is_empty() {
            tags.push("Missed Tactic".to_string());
        }

        if tags.is_empty() {
            tags.push("Positional Advantage".to_string());
        }

        tags.dedup();
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
    }

    #[test]
    fn test_fork_detection() {
        // White Knight on e6 plays Nc7+, forking King on e8 and Rook on a8
        let fen = "r3k3/8/4N3/8/8/8/8/4K3 w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "e1f2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "e6c7", 15, "blunder");
        assert!(tags.contains(&"Fork".to_string()));
    }

    #[test]
    fn test_pin_detection() {
        // White Rook on a1 plays Re1, pinning Black Knight on e6 to Black King on e8
        let fen = "4k3/8/4n3/8/8/8/8/R5K1 w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "g1f2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "a1e1", 15, "blunder");
        assert!(tags.contains(&"Pin".to_string()));
    }

    #[test]
    fn test_skewer_detection() {
        // White Rook on a1 plays Re1+, attacking King on e7 and skewering Queen on e8 behind it
        let fen = "4q3/4k3/8/8/8/8/8/R5K1 w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "g1f2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "a1e1", 15, "blunder");
        assert!(tags.contains(&"Skewer".to_string()));
    }

    #[test]
    fn test_discovered_check_and_attack() {
        // White Rook on e1, Knight on e4, Black King on e8.
        // Nd6+ unblocks e-file from Rook to King (discovered check).
        let fen = "4k3/8/8/8/4N3/8/8/4R1K1 w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "g1f2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "e4c3", 15, "blunder");
        assert!(tags.contains(&"Discovered Check".to_string()));
    }

    #[test]
    fn test_smothered_mate_detection() {
        // Classic smothered mate: Black King on h8 surrounded by g8 Rook, g7 and h7 pawns.
        // White Knight on h6 plays Nf7#
        let fen = "6rk/6pp/7N/8/8/8/8/7K w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "h1h2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "h6f7", 15, "blunder");
        assert!(tags.contains(&"Smothered Mate".to_string()));
    }

    #[test]
    fn test_back_rank_mate_detection() {
        // White Rook on e1 plays Re8#, delivering back-rank mate against trapped King on g8
        let fen = "3r2k1/5ppp/8/8/8/8/8/4R1K1 w - - 0 1";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "g1f2".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "e1e8", 15, "blunder");
        assert!(tags.contains(&"Back Rank".to_string()));
    }

    #[test]
    fn test_hanging_piece_detection() {
        // White Queen on f3 captures undefended Black Knight on e4
        let fen = "r1bqkb1r/pppp1ppp/2n5/4p3/2B1n3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 0 4";
        let pos: Chess = fen.parse::<Fen>().unwrap().into_position(CastlingMode::Standard).unwrap();
        let played_move = "a2a3".parse::<UciMove>().unwrap().to_move(&pos).unwrap();
        let tags = GameAnalyzer::detect_tactical_tags(&pos, &played_move, "f3e4", 15, "blunder");
        assert!(tags.contains(&"Hanging Piece".to_string()));
    }

    #[test]
    fn test_explanation_generation() {
        let fork_tags = vec!["Fork".to_string(), "Tactical Advantage".to_string()];
        let explanation = GameAnalyzer::generate_explanation(&fork_tags, Some("Nf7+"));
        assert!(explanation.contains("Nf7+: Forks multiple enemy pieces"));

        let pin_tags = vec!["Pin".to_string()];
        let explanation = GameAnalyzer::generate_explanation(&pin_tags, None);
        assert!(explanation.contains("Pins an enemy piece"));
    }
}
