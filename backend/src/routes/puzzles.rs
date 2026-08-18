use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::{error, info, warn};

use crate::db;
use crate::models::{PuzzleWithReview, SolveAttemptRequest, SolveResponse};
use crate::routes::sync::AppState;
use crate::srs::Sm2Engine;

#[derive(Deserialize)]
pub struct ReviewQuery {
    pub user_id: i64,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct AllPuzzlesQuery {
    pub user_id: i64,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub severity: Option<String>,
}

pub async fn get_review_queue(
    State(state): State<AppState>,
    Query(query): Query<ReviewQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(20);
    match db::get_due_puzzles(&state.db, query.user_id, limit).await {
        Ok(puzzles) => Ok(Json(puzzles)),
        Err(e) => {
            error!("Error fetching review queue: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn get_all_puzzles(
    State(state): State<AppState>,
    Query(query): Query<AllPuzzlesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let severity = query.severity.as_deref();

    match db::get_all_puzzles(&state.db, query.user_id, limit, offset, severity).await {
        Ok(puzzles) => Ok(Json(puzzles)),
        Err(e) => {
            error!("Error fetching all puzzles: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn get_puzzle_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match db::get_puzzle_with_review(&state.db, id).await {
        Ok(Some(puzzle)) => Ok(Json(puzzle)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Puzzle not found".to_string())),
        Err(e) => {
            error!("Error fetching puzzle {}: {:?}", id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

pub async fn submit_solve(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<SolveAttemptRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let puzzle_with_review = match db::get_puzzle_with_review(&state.db, id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Err((StatusCode::NOT_FOUND, "Puzzle not found".to_string())),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    let review = &puzzle_with_review.review;

    // Calculate quality score
    let quality = if let Some(q) = payload.quality {
        q.min(5)
    } else {
        Sm2Engine::calculate_quality(payload.success, payload.hints_used, payload.time_taken_ms)
    };

    // Calculate next SM-2 schedule
    let srs_result = Sm2Engine::calculate_next_schedule(
        review.easiness_factor,
        review.interval_days,
        review.repetition_number,
        quality,
    );

    // Save update in DB
    if let Err(e) = db::update_puzzle_review(
        &state.db,
        id,
        srs_result.new_ef,
        srs_result.new_interval_days,
        srs_result.new_repetition_number,
        srs_result.next_due_at,
        payload.success,
    )
    .await
    {
        error!("Error updating puzzle review for puzzle {}: {:?}", id, e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }

    Ok(Json(SolveResponse {
        puzzle_id: id,
        success: payload.success,
        new_easiness_factor: srs_result.new_ef,
        new_interval_days: srs_result.new_interval_days,
        new_repetition_number: srs_result.new_repetition_number,
        next_due_at: srs_result.next_due_at,
        is_mastered: srs_result.is_mastered,
    }))
}
