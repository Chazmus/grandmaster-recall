use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite};
use std::collections::HashMap;

use crate::models::{
    OpeningStat, Puzzle, PuzzleReview, PuzzleWithReview, StatsSummary, TacticalTagStat, User,
};

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(database_url: &str) -> Result<DbPool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE,
            platform TEXT NOT NULL DEFAULT 'chess.com',
            last_synced_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS games (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            chess_com_id TEXT NOT NULL UNIQUE,
            time_class TEXT NOT NULL,
            white_player TEXT NOT NULL,
            black_player TEXT NOT NULL,
            result TEXT NOT NULL,
            user_color TEXT NOT NULL,
            pgn TEXT NOT NULL,
            played_at DATETIME NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS puzzles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            game_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            initial_fen TEXT NOT NULL,
            move_number INTEGER NOT NULL,
            player_color TEXT NOT NULL,
            blunder_move_san TEXT NOT NULL,
            blunder_move_uci TEXT NOT NULL,
            best_move_san TEXT NOT NULL,
            best_move_uci TEXT NOT NULL,
            eval_before INTEGER NOT NULL,
            eval_after_blunder INTEGER NOT NULL,
            eval_after_best INTEGER NOT NULL,
            continuation_uci TEXT NOT NULL,
            blunder_continuation_uci TEXT NOT NULL DEFAULT '[]',
            tactical_tags TEXT NOT NULL,
            blunder_severity TEXT NOT NULL,
            opening_name TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS puzzle_reviews (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            puzzle_id INTEGER NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            easiness_factor REAL NOT NULL DEFAULT 2.5,
            interval_days INTEGER NOT NULL DEFAULT 0,
            repetition_number INTEGER NOT NULL DEFAULT 0,
            last_reviewed_at DATETIME,
            next_due_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            times_solved INTEGER NOT NULL DEFAULT 0,
            times_failed INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(puzzle_id) REFERENCES puzzles(id) ON DELETE CASCADE,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_puzzles_user_id ON puzzles(user_id);
        CREATE INDEX IF NOT EXISTS idx_puzzle_reviews_user_due ON puzzle_reviews(user_id, next_due_at);
        CREATE INDEX IF NOT EXISTS idx_games_user_id ON games(user_id);
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_or_create_user(pool: &DbPool, username: &str) -> Result<User> {
    let lower_username = username.to_lowercase();
    
    // Try to get existing user
    let maybe_user = sqlx::query_as::<_, User>(
        "SELECT id, username, platform, last_synced_at, created_at FROM users WHERE lower(username) = lower(?)"
    )
    .bind(&lower_username)
    .fetch_optional(pool)
    .await?;

    if let Some(user) = maybe_user {
        return Ok(user);
    }

    // Insert new user
    let now = Utc::now();
    let id = sqlx::query(
        "INSERT INTO users (username, platform, created_at) VALUES (?, 'chess.com', ?)"
    )
    .bind(&lower_username)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(User {
        id,
        username: lower_username,
        platform: "chess.com".to_string(),
        last_synced_at: None,
        created_at: now,
    })
}

pub async fn update_user_last_synced(pool: &DbPool, user_id: i64) -> Result<()> {
    let now = Utc::now();
    sqlx::query("UPDATE users SET last_synced_at = ? WHERE id = ?")
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_all_users(pool: &DbPool) -> Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, platform, last_synced_at, created_at FROM users ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await?;
    Ok(users)
}

pub async fn count_due_puzzles(pool: &DbPool, user_id: i64) -> Result<i64> {
    let now = Utc::now();
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM puzzle_reviews WHERE user_id = ? AND next_due_at <= ?"
    )
    .bind(user_id)
    .bind(now)
    .fetch_one(pool)
    .await?;
    let count: i64 = row.get("cnt");
    Ok(count)
}

pub async fn count_total_puzzles(pool: &DbPool, user_id: i64) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM puzzles WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let count: i64 = row.get("cnt");
    Ok(count)
}

pub async fn game_exists(pool: &DbPool, chess_com_id: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM games WHERE chess_com_id = ?")
        .bind(chess_com_id)
        .fetch_one(pool)
        .await?;
    let count: i64 = row.get("cnt");
    Ok(count > 0)
}

pub async fn insert_game(
    pool: &DbPool,
    user_id: i64,
    chess_com_id: &str,
    time_class: &str,
    white_player: &str,
    black_player: &str,
    result: &str,
    user_color: &str,
    pgn: &str,
    played_at: DateTime<Utc>,
) -> Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO games (user_id, chess_com_id, time_class, white_player, black_player, result, user_color, pgn, played_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(user_id)
    .bind(chess_com_id)
    .bind(time_class)
    .bind(white_player)
    .bind(black_player)
    .bind(result)
    .bind(user_color)
    .bind(pgn)
    .bind(played_at)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn insert_puzzle(
    pool: &DbPool,
    game_id: i64,
    user_id: i64,
    initial_fen: &str,
    move_number: i32,
    player_color: &str,
    blunder_move_san: &str,
    blunder_move_uci: &str,
    best_move_san: &str,
    best_move_uci: &str,
    eval_before: i32,
    eval_after_blunder: i32,
    eval_after_best: i32,
    continuation_uci: &str,
    blunder_continuation_uci: &str,
    tactical_tags: &str,
    blunder_severity: &str,
    opening_name: Option<&str>,
) -> Result<i64> {
    let mut tx = pool.begin().await?;

    let now = Utc::now();
    let puzzle_id = sqlx::query(
        r#"
        INSERT INTO puzzles (
            game_id, user_id, initial_fen, move_number, player_color,
            blunder_move_san, blunder_move_uci, best_move_san, best_move_uci,
            eval_before, eval_after_blunder, eval_after_best, continuation_uci,
            blunder_continuation_uci, tactical_tags, blunder_severity, opening_name, created_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(game_id)
    .bind(user_id)
    .bind(initial_fen)
    .bind(move_number)
    .bind(player_color)
    .bind(blunder_move_san)
    .bind(blunder_move_uci)
    .bind(best_move_san)
    .bind(best_move_uci)
    .bind(eval_before)
    .bind(eval_after_blunder)
    .bind(eval_after_best)
    .bind(continuation_uci)
    .bind(blunder_continuation_uci)
    .bind(tactical_tags)
    .bind(blunder_severity)
    .bind(opening_name)
    .bind(now)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // Initialize SM-2 review queue row
    sqlx::query(
        r#"
        INSERT INTO puzzle_reviews (
            puzzle_id, user_id, easiness_factor, interval_days,
            repetition_number, last_reviewed_at, next_due_at, times_solved, times_failed
        )
        VALUES (?, ?, 2.5, 0, 0, NULL, ?, 0, 0)
        "#
    )
    .bind(puzzle_id)
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(puzzle_id)
}

pub async fn get_due_puzzles(
    pool: &DbPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<PuzzleWithReview>> {
    let now = Utc::now();
    let rows = sqlx::query(
        r#"
        SELECT 
            p.id, p.game_id, p.user_id, p.initial_fen, p.move_number, p.player_color,
            p.blunder_move_san, p.blunder_move_uci, p.best_move_san, p.best_move_uci,
            p.eval_before, p.eval_after_blunder, p.eval_after_best, p.continuation_uci,
            p.blunder_continuation_uci, p.tactical_tags, p.blunder_severity, p.opening_name, p.created_at,
            r.id as r_id, r.puzzle_id as r_puzzle_id, r.user_id as r_user_id,
            r.easiness_factor, r.interval_days, r.repetition_number,
            r.last_reviewed_at, r.next_due_at, r.times_solved, r.times_failed,
            g.white_player, g.black_player, g.time_class, g.played_at
        FROM puzzles p
        JOIN puzzle_reviews r ON p.id = r.puzzle_id
        JOIN games g ON p.game_id = g.id
        WHERE p.user_id = ? AND r.next_due_at <= ?
        ORDER BY r.next_due_at ASC, p.id DESC
        LIMIT ?
        "#
    )
    .bind(user_id)
    .bind(now)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let puzzle = Puzzle {
            id: row.get("id"),
            game_id: row.get("game_id"),
            user_id: row.get("user_id"),
            initial_fen: row.get("initial_fen"),
            move_number: row.get("move_number"),
            player_color: row.get("player_color"),
            blunder_move_san: row.get("blunder_move_san"),
            blunder_move_uci: row.get("blunder_move_uci"),
            best_move_san: row.get("best_move_san"),
            best_move_uci: row.get("best_move_uci"),
            eval_before: row.get("eval_before"),
            eval_after_blunder: row.get("eval_after_blunder"),
            eval_after_best: row.get("eval_after_best"),
            continuation_uci: row.get("continuation_uci"),
            blunder_continuation_uci: row.get("blunder_continuation_uci"),
            tactical_tags: row.get("tactical_tags"),
            blunder_severity: row.get("blunder_severity"),
            opening_name: row.get("opening_name"),
            created_at: row.get("created_at"),
        };

        let review = PuzzleReview {
            id: row.get("r_id"),
            puzzle_id: row.get("r_puzzle_id"),
            user_id: row.get("r_user_id"),
            easiness_factor: row.get("easiness_factor"),
            interval_days: row.get("interval_days"),
            repetition_number: row.get("repetition_number"),
            last_reviewed_at: row.get("last_reviewed_at"),
            next_due_at: row.get("next_due_at"),
            times_solved: row.get("times_solved"),
            times_failed: row.get("times_failed"),
        };

        result.push(PuzzleWithReview {
            puzzle,
            review,
            game_white: row.get("white_player"),
            game_black: row.get("black_player"),
            game_time_class: row.get("time_class"),
            game_played_at: row.get("played_at"),
        });
    }

    Ok(result)
}

pub async fn get_all_puzzles(
    pool: &DbPool,
    user_id: i64,
    limit: i64,
    offset: i64,
    severity_filter: Option<&str>,
) -> Result<Vec<PuzzleWithReview>> {
    let rows = if let Some(severity) = severity_filter {
        sqlx::query(
            r#"
            SELECT 
                p.id, p.game_id, p.user_id, p.initial_fen, p.move_number, p.player_color,
                p.blunder_move_san, p.blunder_move_uci, p.best_move_san, p.best_move_uci,
                p.eval_before, p.eval_after_blunder, p.eval_after_best, p.continuation_uci,
                p.blunder_continuation_uci, p.tactical_tags, p.blunder_severity, p.opening_name, p.created_at,
                r.id as r_id, r.puzzle_id as r_puzzle_id, r.user_id as r_user_id,
                r.easiness_factor, r.interval_days, r.repetition_number,
                r.last_reviewed_at, r.next_due_at, r.times_solved, r.times_failed,
                g.white_player, g.black_player, g.time_class, g.played_at
            FROM puzzles p
            JOIN puzzle_reviews r ON p.id = r.puzzle_id
            JOIN games g ON p.game_id = g.id
            WHERE p.user_id = ? AND p.blunder_severity = ?
            ORDER BY p.id DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(user_id)
        .bind(severity)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT 
                p.id, p.game_id, p.user_id, p.initial_fen, p.move_number, p.player_color,
                p.blunder_move_san, p.blunder_move_uci, p.best_move_san, p.best_move_uci,
                p.eval_before, p.eval_after_blunder, p.eval_after_best, p.continuation_uci,
                p.blunder_continuation_uci, p.tactical_tags, p.blunder_severity, p.opening_name, p.created_at,
                r.id as r_id, r.puzzle_id as r_puzzle_id, r.user_id as r_user_id,
                r.easiness_factor, r.interval_days, r.repetition_number,
                r.last_reviewed_at, r.next_due_at, r.times_solved, r.times_failed,
                g.white_player, g.black_player, g.time_class, g.played_at
            FROM puzzles p
            JOIN puzzle_reviews r ON p.id = r.puzzle_id
            JOIN games g ON p.game_id = g.id
            WHERE p.user_id = ?
            ORDER BY p.id DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    let mut result = Vec::new();
    for row in rows {
        let puzzle = Puzzle {
            id: row.get("id"),
            game_id: row.get("game_id"),
            user_id: row.get("user_id"),
            initial_fen: row.get("initial_fen"),
            move_number: row.get("move_number"),
            player_color: row.get("player_color"),
            blunder_move_san: row.get("blunder_move_san"),
            blunder_move_uci: row.get("blunder_move_uci"),
            best_move_san: row.get("best_move_san"),
            best_move_uci: row.get("best_move_uci"),
            eval_before: row.get("eval_before"),
            eval_after_blunder: row.get("eval_after_blunder"),
            eval_after_best: row.get("eval_after_best"),
            continuation_uci: row.get("continuation_uci"),
            blunder_continuation_uci: row.get("blunder_continuation_uci"),
            tactical_tags: row.get("tactical_tags"),
            blunder_severity: row.get("blunder_severity"),
            opening_name: row.get("opening_name"),
            created_at: row.get("created_at"),
        };

        let review = PuzzleReview {
            id: row.get("r_id"),
            puzzle_id: row.get("r_puzzle_id"),
            user_id: row.get("r_user_id"),
            easiness_factor: row.get("easiness_factor"),
            interval_days: row.get("interval_days"),
            repetition_number: row.get("repetition_number"),
            last_reviewed_at: row.get("last_reviewed_at"),
            next_due_at: row.get("next_due_at"),
            times_solved: row.get("times_solved"),
            times_failed: row.get("times_failed"),
        };

        result.push(PuzzleWithReview {
            puzzle,
            review,
            game_white: row.get("white_player"),
            game_black: row.get("black_player"),
            game_time_class: row.get("time_class"),
            game_played_at: row.get("played_at"),
        });
    }

    Ok(result)
}

pub async fn get_puzzle_with_review(
    pool: &DbPool,
    puzzle_id: i64,
) -> Result<Option<PuzzleWithReview>> {
    let row = sqlx::query(
        r#"
        SELECT 
            p.id, p.game_id, p.user_id, p.initial_fen, p.move_number, p.player_color,
            p.blunder_move_san, p.blunder_move_uci, p.best_move_san, p.best_move_uci,
            p.eval_before, p.eval_after_blunder, p.eval_after_best, p.continuation_uci,
            p.blunder_continuation_uci, p.tactical_tags, p.blunder_severity, p.opening_name, p.created_at,
            r.id as r_id, r.puzzle_id as r_puzzle_id, r.user_id as r_user_id,
            r.easiness_factor, r.interval_days, r.repetition_number,
            r.last_reviewed_at, r.next_due_at, r.times_solved, r.times_failed,
            g.white_player, g.black_player, g.time_class, g.played_at
        FROM puzzles p
        JOIN puzzle_reviews r ON p.id = r.puzzle_id
        JOIN games g ON p.game_id = g.id
        WHERE p.id = ?
        "#
    )
    .bind(puzzle_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let puzzle = Puzzle {
            id: row.get("id"),
            game_id: row.get("game_id"),
            user_id: row.get("user_id"),
            initial_fen: row.get("initial_fen"),
            move_number: row.get("move_number"),
            player_color: row.get("player_color"),
            blunder_move_san: row.get("blunder_move_san"),
            blunder_move_uci: row.get("blunder_move_uci"),
            best_move_san: row.get("best_move_san"),
            best_move_uci: row.get("best_move_uci"),
            eval_before: row.get("eval_before"),
            eval_after_blunder: row.get("eval_after_blunder"),
            eval_after_best: row.get("eval_after_best"),
            continuation_uci: row.get("continuation_uci"),
            blunder_continuation_uci: row.get("blunder_continuation_uci"),
            tactical_tags: row.get("tactical_tags"),
            blunder_severity: row.get("blunder_severity"),
            opening_name: row.get("opening_name"),
            created_at: row.get("created_at"),
        };

        let review = PuzzleReview {
            id: row.get("r_id"),
            puzzle_id: row.get("r_puzzle_id"),
            user_id: row.get("r_user_id"),
            easiness_factor: row.get("easiness_factor"),
            interval_days: row.get("interval_days"),
            repetition_number: row.get("repetition_number"),
            last_reviewed_at: row.get("last_reviewed_at"),
            next_due_at: row.get("next_due_at"),
            times_solved: row.get("times_solved"),
            times_failed: row.get("times_failed"),
        };

        Ok(Some(PuzzleWithReview {
            puzzle,
            review,
            game_white: row.get("white_player"),
            game_black: row.get("black_player"),
            game_time_class: row.get("time_class"),
            game_played_at: row.get("played_at"),
        }))
    } else {
        Ok(None)
    }
}

pub async fn update_puzzle_review(
    pool: &DbPool,
    puzzle_id: i64,
    easiness_factor: f64,
    interval_days: i32,
    repetition_number: i32,
    next_due_at: DateTime<Utc>,
    success: bool,
) -> Result<()> {
    let now = Utc::now();
    let solved_inc = if success { 1 } else { 0 };
    let failed_inc = if success { 0 } else { 1 };

    sqlx::query(
        r#"
        UPDATE puzzle_reviews
        SET 
            easiness_factor = ?,
            interval_days = ?,
            repetition_number = ?,
            last_reviewed_at = ?,
            next_due_at = ?,
            times_solved = times_solved + ?,
            times_failed = times_failed + ?
        WHERE puzzle_id = ?
        "#
    )
    .bind(easiness_factor)
    .bind(interval_days)
    .bind(repetition_number)
    .bind(now)
    .bind(next_due_at)
    .bind(solved_inc)
    .bind(failed_inc)
    .bind(puzzle_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_stats_summary(pool: &DbPool, user_id: i64) -> Result<StatsSummary> {
    let now = Utc::now();

    // Total puzzles
    let row_total = sqlx::query("SELECT COUNT(*) as cnt FROM puzzles WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let total_puzzles: i64 = row_total.get("cnt");

    // Due today
    let row_due = sqlx::query(
        "SELECT COUNT(*) as cnt FROM puzzle_reviews WHERE user_id = ? AND next_due_at <= ?"
    )
    .bind(user_id)
    .bind(now)
    .fetch_one(pool)
    .await?;
    let due_today: i64 = row_due.get("cnt");

    // Mastered puzzles (interval_days >= 21)
    let row_mastered = sqlx::query(
        "SELECT COUNT(*) as cnt FROM puzzle_reviews WHERE user_id = ? AND interval_days >= 21"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    let mastered_puzzles: i64 = row_mastered.get("cnt");

    // Solve statistics
    let row_solves = sqlx::query(
        "SELECT SUM(times_solved) as solved, SUM(times_failed) as failed FROM puzzle_reviews WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let total_solved: i64 = row_solves.try_get("solved").unwrap_or(0);
    let total_failed: i64 = row_solves.try_get("failed").unwrap_or(0);
    let total_reviews = total_solved + total_failed;
    let retention_rate = if total_reviews > 0 {
        (total_solved as f64 / total_reviews as f64) * 100.0
    } else {
        100.0
    };

    // Severity counts
    let severity_rows = sqlx::query(
        "SELECT blunder_severity, COUNT(*) as cnt FROM puzzles WHERE user_id = ? GROUP BY blunder_severity"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut blunders_count = 0;
    let mut mistakes_count = 0;
    let mut inaccuracies_count = 0;

    for row in severity_rows {
        let sev: String = row.get("blunder_severity");
        let cnt: i64 = row.get("cnt");
        match sev.as_str() {
            "blunder" => blunders_count = cnt,
            "mistake" => mistakes_count = cnt,
            "inaccuracy" => inaccuracies_count = cnt,
            _ => {}
        }
    }

    // Top blundered openings
    let opening_rows = sqlx::query(
        r#"
        SELECT opening_name, COUNT(*) as cnt 
        FROM puzzles 
        WHERE user_id = ? AND opening_name IS NOT NULL AND opening_name != ''
        GROUP BY opening_name 
        ORDER BY cnt DESC 
        LIMIT 5
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let top_blundered_openings: Vec<OpeningStat> = opening_rows
        .into_iter()
        .map(|r| OpeningStat {
            opening_name: r.get("opening_name"),
            blunder_count: r.get("cnt"),
        })
        .collect();

    // Tactical tag stats
    let tag_rows = sqlx::query(
        r#"
        SELECT p.tactical_tags, r.times_solved, r.times_failed
        FROM puzzles p
        JOIN puzzle_reviews r ON p.id = r.puzzle_id
        WHERE p.user_id = ?
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut tag_map: HashMap<String, (i64, i64, i64)> = HashMap::new();
    for row in tag_rows {
        let tags_json: String = row.get("tactical_tags");
        let solved: i64 = row.get("times_solved");
        let failed: i64 = row.get("times_failed");
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_json) {
            for tag in tags {
                let entry = tag_map.entry(tag).or_insert((0, 0, 0));
                entry.0 += 1;
                entry.1 += solved;
                entry.2 += failed;
            }
        }
    }

    let mut tactical_tag_breakdown: Vec<TacticalTagStat> = tag_map
        .into_iter()
        .map(|(tag, (count, solved, failed))| {
            let total = solved + failed;
            let success_rate = if total > 0 {
                (solved as f64 / total as f64) * 100.0
            } else {
                100.0
            };
            TacticalTagStat {
                tag,
                count,
                success_rate,
            }
        })
        .collect();

    tactical_tag_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(StatsSummary {
        total_puzzles,
        due_today,
        mastered_puzzles,
        total_reviews,
        retention_rate,
        blunders_count,
        mistakes_count,
        inaccuracies_count,
        tactical_tag_breakdown,
        top_blundered_openings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> DbPool {
        init_db("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_user_creation_and_lookup() {
        let pool = setup_test_db().await;
        
        let user1 = get_or_create_user(&pool, "Hikaru").await.unwrap();
        assert_eq!(user1.username, "hikaru");

        // Subsequent lookup should return same user
        let user2 = get_or_create_user(&pool, "hikaru").await.unwrap();
        assert_eq!(user1.id, user2.id);
    }

    #[tokio::test]
    async fn test_game_and_puzzle_insertion() {
        let pool = setup_test_db().await;
        let user = get_or_create_user(&pool, "testuser").await.unwrap();

        let game_id = insert_game(
            &pool,
            user.id,
            "https://chess.com/game/123",
            "blitz",
            "testuser",
            "opponent",
            "win",
            "white",
            "1. e4 e5 2. Nf3",
            Utc::now(),
        )
        .await
        .unwrap();

        assert!(game_id > 0);
        assert!(game_exists(&pool, "https://chess.com/game/123").await.unwrap());

        let puzzle_id = insert_puzzle(
            &pool,
            game_id,
            user.id,
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            12,
            "white",
            "Nxd4",
            "f3d4",
            "Qxd4",
            "d1d4",
            100,
            -200,
            100,
            "[\"d1d4\"]",
            "[]",
            "[\"Fork\"]",
            "blunder",
            Some("Sicilian Defense"),
        )
        .await
        .unwrap();

        assert!(puzzle_id > 0);

        // Check review queue
        let due = get_due_puzzles(&pool, user.id, 10).await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].puzzle.id, puzzle_id);
        assert_eq!(due[0].puzzle.blunder_severity, "blunder");

        // Update solve result
        update_puzzle_review(&pool, puzzle_id, 2.6, 1, 1, Utc::now(), true)
            .await
            .unwrap();

        // Check stats
        let stats = get_stats_summary(&pool, user.id).await.unwrap();
        assert_eq!(stats.total_puzzles, 1);
        assert_eq!(stats.blunders_count, 1);
        assert_eq!(stats.retention_rate, 100.0);

        // Check get_all_users and count queries
        let all_users = get_all_users(&pool).await.unwrap();
        assert_eq!(all_users.len(), 1);
        assert_eq!(all_users[0].username, "testuser");

        let total_p = count_total_puzzles(&pool, user.id).await.unwrap();
        assert_eq!(total_p, 1);
    }
}
