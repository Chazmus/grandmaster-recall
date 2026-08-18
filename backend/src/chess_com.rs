use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct ChessComClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct ArchivesResponse {
    pub archives: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MonthlyGamesResponse {
    pub games: Vec<ChessComGame>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChessComPlayer {
    pub username: String,
    pub rating: Option<i32>,
    pub result: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChessComGame {
    pub url: String,
    pub pgn: Option<String>,
    pub time_class: String, // "rapid", "blitz", "bullet", "daily"
    pub rules: String,      // "chess"
    pub white: ChessComPlayer,
    pub black: ChessComPlayer,
    pub end_time: i64,
}

impl ChessComClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("ChessBlunderTrainer/1.0 (cbailey-workspace)")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub async fn verify_user(&self, username: &str) -> Result<String> {
        let url = format!("https://api.chess.com/pub/player/{}", username.trim());
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            return Err(anyhow!("User '{}' not found on Chess.com (HTTP {})", username, res.status()));
        }
        #[derive(Deserialize)]
        struct PlayerProfile {
            username: String,
        }
        let profile = res.json::<PlayerProfile>().await?;
        Ok(profile.username)
    }

    pub async fn fetch_recent_games(
        &self,
        username: &str,
        time_classes: &[String],
        months_back: usize,
        max_games: usize,
    ) -> Result<Vec<ChessComGame>> {
        let clean_username = username.trim().to_lowercase();
        let archives_url = format!("https://api.chess.com/pub/player/{}/games/archives", clean_username);
        let res = self.client.get(&archives_url).send().await?;
        if !res.status().is_success() {
            return Err(anyhow!("Failed to fetch archives for user {}: HTTP {}", clean_username, res.status()));
        }

        let archives: ArchivesResponse = res.json().await?;
        if archives.archives.is_empty() {
            return Ok(Vec::new());
        }

        // Take the latest N monthly archives
        let selected_archives: Vec<String> = archives
            .archives
            .into_iter()
            .rev()
            .take(months_back)
            .collect();

        let mut collected_games = Vec::new();

        for archive_url in selected_archives {
            debug!("Fetching archive from {}", archive_url);
            let resp = match self.client.get(&archive_url).send().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Error fetching archive {}: {:?}", archive_url, e);
                    continue;
                }
            };

            if let Ok(monthly) = resp.json::<MonthlyGamesResponse>().await {
                // Filter by standard chess rules and matching time_classes
                for game in monthly.games.into_iter().rev() {
                    if game.rules != "chess" || game.pgn.is_none() {
                        continue;
                    }
                    if !time_classes.is_empty() && !time_classes.contains(&game.time_class.to_lowercase()) {
                        continue;
                    }
                    collected_games.push(game);
                    if collected_games.len() >= max_games {
                        break;
                    }
                }
            }

            if collected_games.len() >= max_games {
                break;
            }
        }

        info!("Fetched {} qualifying games for {}", collected_games.len(), clean_username);
        Ok(collected_games)
    }
}
