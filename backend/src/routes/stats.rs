use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::error;

use crate::db;
use crate::routes::sync::AppState;

#[derive(Deserialize)]
pub struct StatsQuery {
    pub user_id: i64,
}

pub async fn get_user_stats(
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match db::get_stats_summary(&state.db, query.user_id).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            error!("Error fetching stats for user {}: {:?}", query.user_id, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
