use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::models::{EngineEvalResponse, MoveEval};

#[derive(Clone)]
pub struct EnginePool {
    inner: Arc<Mutex<StockfishWorker>>,
}

struct StockfishWorker {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    _binary_path: PathBuf,
}

impl StockfishWorker {
    async fn new(binary_path: PathBuf) -> Result<Self> {
        let mut child = Command::new(&binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let reader = BufReader::new(stdout);

        let mut worker = Self {
            _child: child,
            stdin,
            reader,
            _binary_path: binary_path,
        };

        worker.send_command("uci").await?;
        worker.wait_for("uciok").await?;
        worker.send_command("setoption name Threads value 2").await?;
        worker.send_command("setoption name Hash value 64").await?;
        worker.send_command("isready").await?;
        worker.wait_for("readyok").await?;

        Ok(worker)
    }

    async fn send_command(&mut self, cmd: &str) -> Result<()> {
        let line = format!("{}\n", cmd);
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn wait_for(&mut self, token: &str) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let mut buf = String::new();
        loop {
            buf.clear();
            let bytes = self.reader.read_line(&mut buf).await?;
            if bytes == 0 {
                return Err(anyhow!("Stockfish process EOF while waiting for '{}'", token));
            }
            let trimmed = buf.trim().to_string();
            if trimmed.contains(token) {
                lines.push(trimmed);
                break;
            }
            lines.push(trimmed);
        }
        Ok(lines)
    }

    async fn evaluate_fen_internal(
        &mut self,
        fen: &str,
        depth: u32,
        multi_pv: u32,
    ) -> Result<EngineEvalResponse> {
        self.send_command("ucinewgame").await?;
        self.send_command("isready").await?;
        self.wait_for("readyok").await?;

        self.send_command(&format!("setoption name MultiPV value {}", multi_pv)).await?;
        self.send_command(&format!("position fen {}", fen)).await?;
        self.send_command(&format!("go depth {}", depth)).await?;

        let mut lines_map: std::collections::BTreeMap<u32, MoveEval> = std::collections::BTreeMap::new();
        let mut best_move = String::new();
        let mut best_cp: Option<i32> = None;
        let mut best_mate: Option<i32> = None;

        let mut buf = String::new();
        loop {
            buf.clear();
            let bytes = self.reader.read_line(&mut buf).await?;
            if bytes == 0 {
                return Err(anyhow!("Stockfish EOF during evaluation"));
            }
            let line = buf.trim();

            if line.starts_with("bestmove ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    best_move = parts[1].to_string();
                }
                break;
            }

            if line.starts_with("info ") && line.contains(" pv ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut pv_index = 1;
                let mut cp: Option<i32> = None;
                let mut mate: Option<i32> = None;
                let mut pv_moves: Vec<String> = Vec::new();

                let mut i = 0;
                while i < parts.len() {
                    match parts[i] {
                        "multipv" if i + 1 < parts.len() => {
                            if let Ok(val) = parts[i + 1].parse::<u32>() {
                                pv_index = val;
                            }
                            i += 2;
                        }
                        "score" if i + 2 < parts.len() => {
                            match parts[i + 1] {
                                "cp" => {
                                    cp = parts[i + 2].parse::<i32>().ok();
                                }
                                "mate" => {
                                    mate = parts[i + 2].parse::<i32>().ok();
                                }
                                _ => {}
                            }
                            i += 3;
                        }
                        "pv" => {
                            for move_str in &parts[(i + 1)..] {
                                pv_moves.push(move_str.to_string());
                            }
                            break;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }

                if !pv_moves.is_empty() {
                    let first_move = pv_moves[0].clone();
                    if pv_index == 1 {
                        best_cp = cp;
                        best_mate = mate;
                    }
                    lines_map.insert(
                        pv_index,
                        MoveEval {
                            uci: first_move,
                            san: None,
                            score_cp: cp,
                            mate_in: mate,
                            pv: pv_moves,
                        },
                    );
                }
            }
        }

        let lines: Vec<MoveEval> = lines_map.into_values().collect();

        Ok(EngineEvalResponse {
            fen: fen.to_string(),
            depth,
            best_move,
            score_cp: best_cp,
            mate_in: best_mate,
            lines,
        })
    }
}

impl EnginePool {
    pub async fn new(binary_path: PathBuf) -> Result<Self> {
        info!("Initializing Stockfish engine at {:?}", binary_path);
        let worker = StockfishWorker::new(binary_path).await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(worker)),
        })
    }

    pub async fn evaluate_fen(
        &self,
        fen: &str,
        depth: u32,
        multi_pv: u32,
    ) -> Result<EngineEvalResponse> {
        let mut worker = self.inner.lock().await;
        worker.evaluate_fen_internal(fen, depth, multi_pv).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stockfish_engine_evaluation() {
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
            PathBuf::from("/home/cbailey/workspace/chess-trainer/engine/stockfish/stockfish-linux-x86-64-universal"),
            PathBuf::from("/usr/bin/stockfish"),
            PathBuf::from("/usr/local/bin/stockfish"),
        ]);

        let path = match possible_paths.into_iter().find(|p| p.exists()) {
            Some(p) => p,
            None => return,
        };

        let engine = EnginePool::new(path).await.unwrap();
        // Scholar's mate threat test FEN: White to move, Qxf7# is mate in 1
        let fen = "r1bqkb1r/pppp1ppp/2n5/4p3/2B1n3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 0 4";
        let res = engine.evaluate_fen(fen, 10, 1).await.unwrap();

        assert_eq!(res.best_move, "f3f7");
        assert!(res.mate_in == Some(1) || res.score_cp.unwrap_or(0) > 800);
        assert!(!res.lines.is_empty());
    }
}
