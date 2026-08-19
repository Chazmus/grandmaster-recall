use anyhow::Result;
use chrono::{TimeZone, Utc};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::analyzer::GameAnalyzer;
use crate::db;
use crate::models::{SyncStatus, User};
use crate::routes::sync::AppState;

pub const LOW_WATERMARK: i64 = 8;
pub const HIGH_WATERMARK: i64 = 16;
pub const DAEMON_INTERVAL_SECS: u64 = 15 * 60; // 15 minutes
pub const THROTTLE_SLEEP_MS: u64 = 100; // 100ms cooldown between move evaluations to keep CPU cool
pub const BACKGROUND_EVAL_DEPTH: u32 = 16; // High depth analysis for strong tactical accuracy on Pi 5

pub async fn run_background_puzzle_daemon(state: AppState) {
    info!(
        "Starting Watermark Background Puzzle Daemon (Low={}, High={}, Interval={}m, Throttle={}ms)...",
        LOW_WATERMARK,
        HIGH_WATERMARK,
        DAEMON_INTERVAL_SECS / 60,
        THROTTLE_SLEEP_MS
    );

    // Initial brief delay on startup
    tokio::time::sleep(Duration::from_secs(8)).await;

    loop {
        if let Err(e) = replenish_all_buffers(&state).await {
            error!("Error in background puzzle daemon: {:?}", e);
        }

        tokio::time::sleep(Duration::from_secs(DAEMON_INTERVAL_SECS)).await;
    }
}

async fn replenish_all_buffers(state: &AppState) -> Result<()> {
    let users = db::get_all_users(&state.db).await?;
    if users.is_empty() {
        debug!("Background daemon: No registered users found yet.");
        return Ok(());
    }

    for user in users {
        if let Err(e) = replenish_user_buffer(state, &user).await {
            warn!("Background daemon: Failed replenishment for user {}: {:?}", user.username, e);
        }
    }

    Ok(())
}

pub async fn replenish_user_buffer(state: &AppState, user: &User) -> Result<usize> {
    let lower = user.username.to_lowercase();

    // 1. Check if user is currently running a manual sync
    {
        let lock = state.sync_states.lock().await;
        if let Some(st) = lock.get(&lower) {
            if st.state == "fetching_games" || st.state == "analyzing" {
                debug!("Background daemon: Skipping user {} as manual sync is in progress.", user.username);
                return Ok(0);
            }
        }
    }

    // 2. Check due puzzles count (watermark check)
    let due_count = db::count_due_puzzles(&state.db, user.id).await.unwrap_or(0);
    let total_count = db::count_total_puzzles(&state.db, user.id).await.unwrap_or(0);

    // If user already has enough due puzzles and total puzzles, skip quietly (0% CPU)
    if due_count >= LOW_WATERMARK && total_count >= HIGH_WATERMARK {
        debug!(
            "Background daemon: User {} has {} due puzzles (total {}). Buffer is healthy.",
            user.username, due_count, total_count
        );
        return Ok(0);
    }

    debug!(
        "Background daemon: User {} queue below watermark (due: {}, total: {}). Quietly replenishing buffer...",
        user.username, due_count, total_count
    );

    // Record that we are fetching games
    {
        let mut lock = state.sync_states.lock().await;
        lock.insert(
            lower.clone(),
            SyncStatus {
                username: user.username.clone(),
                state: "fetching_games".to_string(),
                total_games: 0,
                processed_games: 0,
                puzzles_found: 0,
                current_game: None,
                error: None,
            },
        );
    }

    // 3. Fetch small batch of recent games (1-3 games)
    let recent_games = match state
        .chess_com
        .fetch_recent_games(&user.username, &["rapid".to_string(), "blitz".to_string()], 1, 3)
        .await
    {
        Ok(g) => g,
        Err(e) => {
            warn!("Background daemon: Failed to fetch recent games for {}: {:?}", user.username, e);
            let mut lock = state.sync_states.lock().await;
            if let Some(st) = lock.get_mut(&lower) {
                st.state = "idle".to_string();
                st.error = Some(e.to_string());
            }
            return Ok(0);
        }
    };

    if recent_games.is_empty() {
        let mut lock = state.sync_states.lock().await;
        if let Some(st) = lock.get_mut(&lower) {
            st.state = "idle".to_string();
        }
        return Ok(0);
    }

    let total_games = recent_games.len();
    {
        let mut lock = state.sync_states.lock().await;
        if let Some(st) = lock.get_mut(&lower) {
            st.state = "analyzing".to_string();
            st.total_games = total_games;
        }
    }

    let analyzer = GameAnalyzer::new(state.engine.clone());
    let mut puzzles_created = 0;
    let mut current_due = due_count;

    for (idx, game) in recent_games.iter().enumerate() {
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

        // Update progress
        {
            let mut lock = state.sync_states.lock().await;
            if let Some(st) = lock.get_mut(&lower) {
                st.processed_games = idx + 1;
                st.current_game = Some(format!(
                    "vs {} ({})",
                    if user_color == "white" { black_player } else { white_player },
                    game.time_class
                ));
                st.puzzles_found = puzzles_created;
            }
        }

        let already_exists = db::game_exists(&state.db, game_url).await.unwrap_or(false);
        if already_exists {
            continue;
        }

        // Insert game into DB
        let game_id = match db::insert_game(
            &state.db,
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
                warn!("Background daemon: Failed to insert game {}: {:?}", game_url, e);
                continue;
            }
        };

        // Run throttled analysis (depth 16, 100ms inter-move delay)
        let detected = match analyzer
            .analyze_game_blunders_throttled(
                pgn,
                &user.username,
                white_player,
                black_player,
                BACKGROUND_EVAL_DEPTH,
                THROTTLE_SLEEP_MS,
            )
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!("Background daemon: Error analyzing game {}: {:?}", game_url, e);
                continue;
            }
        };

        for p in detected {
            let cont_json = serde_json::to_string(&p.continuation_uci).unwrap_or_else(|_| "[]".to_string());
            let blunder_cont_json = serde_json::to_string(&p.blunder_continuation_uci).unwrap_or_else(|_| "[]".to_string());
            let tags_json = serde_json::to_string(&p.tactical_tags).unwrap_or_else(|_| "[]".to_string());

            if (db::insert_puzzle(
                &state.db,
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
            .await)
                .is_ok()
            {
                puzzles_created += 1;
                current_due += 1;
                let mut lock = state.sync_states.lock().await;
                if let Some(st) = lock.get_mut(&lower) {
                    st.puzzles_found = puzzles_created;
                }
            }
        }

        // If we have refilled the queue up to the high watermark, stop processing further games
        if current_due >= HIGH_WATERMARK {
            break;
        }
    }

    let _ = db::update_user_last_synced(&state.db, user.id).await;

    // Mark completion
    {
        let mut lock = state.sync_states.lock().await;
        if let Some(st) = lock.get_mut(&lower) {
            st.state = "completed".to_string();
            st.processed_games = total_games;
            st.puzzles_found = puzzles_created;
            st.current_game = None;
        }
    }

    if puzzles_created > 0 {
        info!(
            "Background daemon: Successfully added {} fresh blunder puzzles for user {}.",
            puzzles_created, user.username
        );
    }

    Ok(puzzles_created)
}
