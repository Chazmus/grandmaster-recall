use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::analyzer::GameAnalyzer;
use crate::chess_com::ChessComClient;
use crate::db::{self, DbPool};
use crate::engine::EnginePool;
use crate::models::{SyncRequest, SyncStatus, User};

pub type SyncStateMap = Arc<Mutex<HashMap<String, SyncStatus>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub engine: EnginePool,
    pub chess_com: ChessComClient,
    pub sync_states: SyncStateMap,
}

#[derive(Deserialize)]
pub struct SyncStatusQuery {
    pub username: String,
}

pub async fn start_sync(
    State(state): State<AppState>,
    Json(payload): Json<SyncRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let raw_username = payload.username.trim().to_string();
    if raw_username.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Username cannot be empty".to_string()));
    }

    // Verify user on Chess.com
    let verified_username = match state.chess_com.verify_user(&raw_username).await {
        Ok(u) => u,
        Err(e) => return Err((StatusCode::NOT_FOUND, e.to_string())),
    };

    let lower_username = verified_username.to_lowercase();

    // Check if sync already running
    {
        let mut lock = state.sync_states.lock().await;
        if let Some(st) = lock.get(&lower_username) {
            if st.state == "fetching_games" || st.state == "analyzing" {
                return Ok(Json(st.clone()));
            }
        }

        lock.insert(
            lower_username.clone(),
            SyncStatus {
                username: verified_username.clone(),
                state: "fetching_games".to_string(),
                total_games: 0,
                processed_games: 0,
                puzzles_found: 0,
                current_game: None,
                error: None,
            },
        );
    }

    // Spawn background task
    let bg_state = state.clone();
    let bg_username = verified_username.clone();
    let time_classes = payload.time_classes.unwrap_or_else(|| vec!["rapid".to_string(), "blitz".to_string()]);
    let max_games = payload.max_games.unwrap_or(25);
    let months_back = payload.months_back.unwrap_or(2);
    let engine_depth = payload.engine_depth.unwrap_or(13);

    tokio::spawn(async move {
        let lower = bg_username.to_lowercase();
        
        // 1. Get or create user in DB
        let user = match db::get_or_create_user(&bg_state.db, &bg_username).await {
            Ok(u) => u,
            Err(e) => {
                error!("DB error creating user {}: {:?}", bg_username, e);
                let mut lock = bg_state.sync_states.lock().await;
                if let Some(st) = lock.get_mut(&lower) {
                    st.state = "failed".to_string();
                    st.error = Some(e.to_string());
                }
                return;
            }
        };

        // 2. Fetch games from Chess.com
        info!("Fetching games for {} (time_classes={:?}, max={})", bg_username, time_classes, max_games);
        let games = match bg_state.chess_com.fetch_recent_games(&bg_username, &time_classes, months_back, max_games).await {
            Ok(g) => g,
            Err(e) => {
                error!("Chess.com fetch error for {}: {:?}", bg_username, e);
                let mut lock = bg_state.sync_states.lock().await;
                if let Some(st) = lock.get_mut(&lower) {
                    st.state = "failed".to_string();
                    st.error = Some(e.to_string());
                }
                return;
            }
        };

        let total_games = games.len();
        {
            let mut lock = bg_state.sync_states.lock().await;
            if let Some(st) = lock.get_mut(&lower) {
                st.state = "analyzing".to_string();
                st.total_games = total_games;
            }
        }

        let analyzer = GameAnalyzer::new(bg_state.engine.clone());
        let mut total_puzzles_created = 0;

        for (idx, game) in games.iter().enumerate() {
            let pgn = match &game.pgn {
                Some(p) => p,
                None => continue,
            };

            let game_url = &game.url;
            let white_player = &game.white.username;
            let black_player = &game.black.username;
            let played_at = Utc.timestamp_opt(game.end_time, 0).single().unwrap_or_else(Utc::now);
            let user_color = if white_player.to_lowercase() == lower { "white" } else { "black" };
            let result = if user_color == "white" { &game.white.result } else { &game.black.result };

            // Update status
            {
                let mut lock = bg_state.sync_states.lock().await;
                if let Some(st) = lock.get_mut(&lower) {
                    st.processed_games = idx + 1;
                    st.current_game = Some(format!("vs {} ({})", if user_color == "white" { black_player } else { white_player }, game.time_class));
                    st.puzzles_found = total_puzzles_created;
                }
            }

            // Check if game already analyzed
            let already_exists = match db::game_exists(&bg_state.db, game_url).await {
                Ok(exists) => exists,
                Err(_) => false,
            };

            if already_exists {
                continue;
            }

            // Insert game
            let game_id = match db::insert_game(
                &bg_state.db,
                user.id,
                game_url,
                &game.time_class,
                white_player,
                black_player,
                result,
                user_color,
                pgn,
                played_at,
            )
            .await
            {
                Ok(id) => id,
                Err(e) => {
                    warn!("Failed to insert game {}: {:?}", game_url, e);
                    continue;
                }
            };

            // Run blunder & tactical puzzle detection
            match analyzer.analyze_game_blunders(pgn, &bg_username, white_player, black_player, engine_depth).await {
                Ok(puzzles) => {
                    for p in puzzles {
                        let cont_json = serde_json::to_string(&p.continuation_uci).unwrap_or_else(|_| "[]".to_string());
                        let blunder_cont_json = serde_json::to_string(&p.blunder_continuation_uci).unwrap_or_else(|_| "[]".to_string());
                        let tags_json = serde_json::to_string(&p.tactical_tags).unwrap_or_else(|_| "[]".to_string());

                        match db::insert_puzzle(
                            &bg_state.db,
                            game_id,
                            user.id,
                            &p.initial_fen,
                            p.move_number,
                            &p.player_color,
                            &p.blunder_move_san,
                            &p.blunder_move_uci,
                            &p.best_move_san,
                            &p.best_move_uci,
                            p.eval_before,
                            p.eval_after_blunder,
                            p.eval_after_best,
                            &cont_json,
                            &blunder_cont_json,
                            &tags_json,
                            &p.blunder_severity,
                            p.opening_name.as_deref(),
                        )
                        .await
                        {
                            Ok(_) => {
                                total_puzzles_created += 1;
                            }
                            Err(e) => {
                                warn!("Failed to insert puzzle: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to analyze game {}: {:?}", game_url, e);
                }
            }
        }

        // Update user last synced
        let _ = db::update_user_last_synced(&bg_state.db, user.id).await;

        // Mark completed
        {
            let mut lock = bg_state.sync_states.lock().await;
            if let Some(st) = lock.get_mut(&lower) {
                st.state = "completed".to_string();
                st.processed_games = total_games;
                st.puzzles_found = total_puzzles_created;
                st.current_game = None;
            }
        }
        info!("Sync complete for {}. Created {} puzzles.", bg_username, total_puzzles_created);
    });

    let lock = state.sync_states.lock().await;
    let current_status = lock.get(&lower_username).cloned().unwrap();
    Ok(Json(current_status))
}

pub async fn get_sync_status(
    State(state): State<AppState>,
    Query(query): Query<SyncStatusQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let lower = query.username.trim().to_lowercase();
    let lock = state.sync_states.lock().await;
    if let Some(st) = lock.get(&lower) {
        Ok(Json(st.clone()))
    } else {
        Ok(Json(SyncStatus {
            username: query.username,
            state: "idle".to_string(),
            total_games: 0,
            processed_games: 0,
            puzzles_found: 0,
            current_game: None,
            error: None,
        }))
    }
}
