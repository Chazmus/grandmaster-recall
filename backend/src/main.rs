use axum::{
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod analyzer;
mod background;
mod chess_com;
mod db;
mod engine;
mod models;
mod routes;
mod srs;

use chess_com::ChessComClient;
use engine::EnginePool;
use routes::sync::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("Starting Chess Blunder Trainer Backend...");

    // 2. Setup SQLite Database directory & URL
    let db_dir_str = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let db_dir = Path::new(&db_dir_str);
    if !db_dir.exists() {
        tokio::fs::create_dir_all(db_dir).await?;
    }
    let db_path = db_dir.join("chess_trainer.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    info!("Connecting to SQLite database: {}", db_url);
    let pool = db::init_db(&db_url).await?;

    // 3. Initialize Stockfish Engine
    let mut possible_paths = Vec::new();
    if let Ok(env_path) = std::env::var("STOCKFISH_PATH") {
        possible_paths.push(PathBuf::from(env_path));
    }
    possible_paths.extend(vec![
        PathBuf::from("../engine/stockfish/stockfish-linux-arm64-universal"),
        PathBuf::from("./engine/stockfish/stockfish-linux-arm64-universal"),
        PathBuf::from("../engine/stockfish/stockfish-linux-x86-64-universal"),
        PathBuf::from("./engine/stockfish/stockfish-linux-x86-64-universal"),
        PathBuf::from("/home/cbailey/grandmaster-recall/engine/stockfish/stockfish-linux-arm64-universal"),
        PathBuf::from("/home/cbailey/grandmaster-recall/engine/stockfish/stockfish-linux-x86-64-universal"),
        PathBuf::from("/usr/bin/stockfish"),
        PathBuf::from("/usr/local/bin/stockfish"),
        PathBuf::from("/home/cbailey/workspace/chess-trainer/engine/stockfish/stockfish-linux-x86-64-universal"),
    ]);

    let engine_path = possible_paths
        .into_iter()
        .find(|p| p.exists())
        .expect("Stockfish binary not found in expected engine directory! Set STOCKFISH_PATH environment variable.");

    info!("Using Stockfish binary at: {:?}", engine_path);
    let engine = EnginePool::new(engine_path).await?;

    // 4. Setup application state
    let state = AppState {
        db: pool,
        engine,
        chess_com: ChessComClient::new(),
        sync_states: Arc::new(Mutex::new(HashMap::new())),
    };

    // 5. Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 6. Define Routes
    let mut app = Router::new()
        // User routes
        .route("/api/users/profile", get(routes::users::get_or_create_user))
        // Sync routes
        .route("/api/sync", post(routes::sync::start_sync))
        .route("/api/sync/status", get(routes::sync::get_sync_status))
        // Puzzle & Review routes
        .route("/api/puzzles/review", get(routes::puzzles::get_review_queue))
        .route("/api/puzzles/all", get(routes::puzzles::get_all_puzzles))
        .route("/api/puzzles/:id", get(routes::puzzles::get_puzzle_by_id))
        .route("/api/puzzles/:id/solve", post(routes::puzzles::submit_solve))
        // Engine evaluation & move validation routes
        .route("/api/engine/evaluate", post(routes::engine::evaluate_position))
        .route("/api/engine/validate_move", post(routes::engine::validate_move))
        // Stats route
        .route("/api/stats", get(routes::stats::get_user_stats))
        .layer(cors)
        .with_state(state.clone());

    // Fallback: serve production static frontend assets if dist exists
    let dist_candidates = vec![
        std::env::var("DIST_DIR").ok().map(PathBuf::from),
        Some(PathBuf::from("dist")),
        Some(PathBuf::from("../frontend/dist")),
        Some(PathBuf::from("frontend/dist")),
    ];
    if let Some(dist_path) = dist_candidates.into_iter().flatten().find(|p| p.exists() && p.join("index.html").exists()) {
        info!("Serving static frontend assets from {:?}", dist_path);
        let serve_dir = tower_http::services::ServeDir::new(&dist_path)
            .fallback(tower_http::services::ServeFile::new(dist_path.join("index.html")));
        app = app.fallback_service(serve_dir);
    }

    // 7. Spawn Watermark Background Puzzle Daemon
    let daemon_state = state;
    tokio::spawn(async move {
        background::run_background_puzzle_daemon(daemon_state).await;
    });

    // 8. Bind & Serve
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
