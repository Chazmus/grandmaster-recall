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
pub struct UserQuery {
    pub username: String,
}

pub async fn get_or_create_user(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let clean = query.username.trim();
    if clean.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Username required".to_string()));
    }

    match db::get_or_create_user(&state.db, clean).await {
        Ok(user) => {
            // Gently check buffer in background
            let bg_state = state.clone();
            let bg_user = user.clone();
            tokio::spawn(async move {
                let _ = crate::background::replenish_user_buffer(&bg_state, &bg_user).await;
            });

            Ok(Json(user))
        }
        Err(e) => {
            error!("Error in get_or_create_user: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
