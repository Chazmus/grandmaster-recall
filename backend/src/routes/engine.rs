use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess, Position};
use tracing::error;

use crate::analyzer::GameAnalyzer;
use crate::models::{EngineEvalRequest, ValidateMoveRequest, ValidateMoveResponse};
use crate::routes::sync::AppState;

pub async fn evaluate_position(
    State(state): State<AppState>,
    Json(payload): Json<EngineEvalRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let depth = payload.depth.unwrap_or(14).min(24);
    let multi_pv = payload.multi_pv.unwrap_or(3).min(5);

    match state.engine.evaluate_fen(&payload.fen, depth, multi_pv).await {
        Ok(eval_res) => Ok(Json(eval_res)),
        Err(e) => {
            error!("Engine evaluation error on FEN {}: {:?}", payload.fen, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn validate_move(
    State(state): State<AppState>,
    Json(payload): Json<ValidateMoveRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let clean_move = payload.move_uci.trim().to_lowercase();
    let expected_best = payload.expected_best_uci.trim().to_lowercase();
    let is_white = payload.player_color.to_lowercase() == "white";

    let mut pos: Chess = match payload.fen.parse::<Fen>() {
        Ok(f) => match f.into_position(shakmaty::CastlingMode::Standard) {
            Ok(p) => p,
            Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Invalid FEN: {:?}", e))),
        },
        Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Invalid FEN format: {:?}", e))),
    };

    let move_parsed: shakmaty::Move = match clean_move.parse::<UciMove>() {
        Ok(u) => match u.to_move(&pos) {
            Ok(m) => m,
            Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Illegal move: {:?}", e))),
        },
        Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Invalid UCI move: {:?}", e))),
    };

    // If exact best move match
    if clean_move == expected_best || clean_move.starts_with(&expected_best.chars().take(4).collect::<String>()) {
        let mut pos_after = pos.clone();
        pos_after.play_unchecked(&move_parsed);
        let fen_after = Fen::from_position(pos_after, shakmaty::EnPassantMode::Legal).to_string();

        let opponent_reply = match state.engine.evaluate_fen(&fen_after, 12, 1).await {
            Ok(res) if !res.best_move.is_empty() => Some(res.best_move),
            _ => None,
        };

        return Ok(Json(ValidateMoveResponse {
            is_valid: true,
            is_best: true,
            eval_diff_cp: 0,
            explanation: "Brilliant! Best move.".to_string(),
            opponent_reply_uci: opponent_reply,
        }));
    }

    // Evaluate position before move
    let eval_before_res = match state.engine.evaluate_fen(&payload.fen, 13, 2).await {
        Ok(r) => r,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    // Play user move and evaluate position after
    let mut pos_after = pos.clone();
    pos_after.play_unchecked(&move_parsed);
    let fen_after = Fen::from_position(pos_after, shakmaty::EnPassantMode::Legal).to_string();

    let eval_after_res = match state.engine.evaluate_fen(&fen_after, 13, 1).await {
        Ok(r) => r,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    let eval_before_pov = GameAnalyzer::score_from_pov(
        eval_before_res.score_cp,
        eval_before_res.mate_in,
        is_white,
    );

    let eval_after_pov = GameAnalyzer::score_from_pov(
        eval_after_res.score_cp,
        eval_after_res.mate_in,
        !is_white,
    );

    let eval_diff = eval_before_pov - eval_after_pov;

    // Flexible acceptance criteria:
    // 1. Eval drop is negligible (<= 35 cp) OR
    // 2. Both before and after evaluations retain a winning advantage (>= +220 cp)
    let is_valid = eval_diff <= 35 || (eval_before_pov >= 220 && eval_after_pov >= 180);

    let opponent_reply = if is_valid && !eval_after_res.best_move.is_empty() {
        Some(eval_after_res.best_move)
    } else {
        None
    };

    let explanation = if is_valid {
        if eval_diff <= 15 {
            "Excellent move! Equally strong alternative.".to_string()
        } else {
            format!("Good move! (Stockfish preferred other line, but this retains a +{:.1} advantage)", (eval_after_pov as f64 / 100.0).abs())
        }
    } else {
        "Not quite. This move concedes the advantage. Try again!".to_string()
    };

    Ok(Json(ValidateMoveResponse {
        is_valid,
        is_best: false,
        eval_diff_cp: eval_diff,
        explanation,
        opponent_reply_uci: opponent_reply,
    }))
}
