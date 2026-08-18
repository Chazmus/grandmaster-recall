use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::error;

use crate::models::EngineEvalRequest;
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
