#!/usr/bin/env bash
set -e

echo "=================================================="
echo "    Grandmaster Recall — Chess Blunder Trainer    "
echo "=================================================="

export PATH="$HOME/.cargo/bin:$PATH"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure SQLite data directory exists
mkdir -p "$PROJECT_DIR/backend/data"

# Auto-download Stockfish binary if not present
ENGINE_DIR="$PROJECT_DIR/engine"
ARCH="$(uname -m)"
if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
    SF_ARCH="arm64"
else
    SF_ARCH="x86-64"
fi
STOCKFISH_BIN="$ENGINE_DIR/stockfish/stockfish-linux-${SF_ARCH}-universal"

if [ ! -f "$STOCKFISH_BIN" ]; then
    echo "-> Stockfish binary not found. Downloading Stockfish for Linux ${SF_ARCH}..."
    mkdir -p "$ENGINE_DIR"
    cd "$ENGINE_DIR"
    curl -L -o stockfish.tar.gz "https://github.com/official-stockfish/Stockfish/releases/download/stockfish-dev-20260810-5062aee5/stockfish-linux-${SF_ARCH}-universal.tar.gz"
    tar -xzf stockfish.tar.gz
    rm -f stockfish.tar.gz
    chmod +x "$STOCKFISH_BIN"
    echo "-> Stockfish installed successfully!"
fi

# Trap SIGINT/SIGTERM to kill all child processes
trap 'kill $(jobs -p) 2>/dev/null || true' EXIT SIGINT SIGTERM

echo "-> Starting Rust backend (Axum + Stockfish UCI on port 3001)..."
(cd "$PROJECT_DIR/backend" && cargo run) &
BACKEND_PID=$!

echo "-> Starting Vite frontend on port 5173..."
(cd "$PROJECT_DIR/frontend" && npm run dev -- --host) &
FRONTEND_PID=$!

echo "=================================================="
echo " App running at: http://localhost:5173"
echo " Backend API at: http://localhost:3001/api"
echo " Press Ctrl+C to stop both servers."
echo "=================================================="

wait
