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
    let db_dir = Path::new("data");
    if !db_dir.exists() {
        tokio::fs::create_dir_all(db_dir).await?;
    }
    let db_path = db_dir.join("chess_trainer.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
    info!("Connecting to SQLite database: {}", db_url);
    let pool = db::init_db(&db_url).await?;

    // 3. Initialize Stockfish Engine
    let possible_paths = vec![
        PathBuf::from("../engine/stockfish/stockfish-linux-x86-64-universal"),
        PathBuf::from("./engine/stockfish/stockfish-linux-x86-64-universal"),
        PathBuf::from("/home/cbailey/workspace/chess-trainer/engine/stockfish/stockfish-linux-x86-64-universal"),
    ];

    let engine_path = possible_paths
        .into_iter()
        .find(|p| p.exists())
        .expect("Stockfish binary not found in expected engine directory!");

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
    let app = Router::new()
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
        // Engine evaluation route
        .route("/api/engine/evaluate", post(routes::engine::evaluate_position))
        // Stats route
        .route("/api/stats", get(routes::stats::get_user_stats))
        .layer(cors)
        .with_state(state);

    // 7. Bind & Serve
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
