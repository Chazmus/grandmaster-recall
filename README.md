# Grandmaster Recall — Personal Chess Blunder Trainer

An intelligent chess training web application that connects to your **Chess.com** account, analyzes your historical games with **Stockfish 17+** in Rust to pinpoint your inaccuracies, mistakes, and blunders, and transforms them into personal tactical puzzles scheduled via the **SM-2 Spaced Repetition Algorithm**.

---

## ⚡ Key Features

1. **Chess.com Archive Ingestion**:
   - Fetch historical games by username with configurable time controls (Rapid, Blitz, Bullet, Daily).
   - Real-time progress bar tracking game analysis and blunders found.

2. **Ultra-Fast Stockfish UCI Engine in Rust**:
   - Asynchronous worker process pool in Rust (Axum + Tokio).
   - Position-by-position evaluation detecting centipawn drops, tactical misses, and win-rate swings.
   - Multi-move defensive playout: After you find the first winning move, the engine plays out the defensive replies so you can finish the full tactical combination.
   - *"Why was my move bad?"* punishment demonstrator showing how the opponent could have exploited your mistake.

3. **SM-2 Spaced Repetition Scheduler**:
   - Implements SuperMemo-2 adaptive scheduling ($EF$, repetition count, interval calculation).
   - Rate your recall after solving (*Again*, *Hard*, *Good*, *Easy*) to tune your personal review interval.
   - Review queue prioritizing overdue and due puzzles daily.

4. **Modern Tactical Web Interface**:
   - Authentic **Chessground** (Lichess board engine) with responsive drag-and-drop and touch support.
   - Official Lichess high-res piece sets (`cburnett`) and sound effects (`Move`, `Capture`, `Check`, `Victory`, `Defeat`, `Error`).
   - Tactical motif breakdown (Forks, Pins, Hanging Pieces, Back Rank, Opening Mistakes, Endgame Blunders).
   - Opening weakness breakdown showing which repertoires cause you the most tactical trouble.

---

## 🚀 Quick Start

### Option 1: Run with Docker (Fastest & Recommended)

Pre-built multi-architecture images (`linux/amd64` and `linux/arm64` / Raspberry Pi / Apple Silicon) are automatically published to the GitHub Container Registry.

#### Using Docker Compose:
```bash
docker compose up -d
```

#### Or with `docker run`:
```bash
docker run -d \
  --name grandmaster-recall \
  -p 3001:3001 \
  -v $(pwd)/data:/app/data \
  --restart unless-stopped \
  ghcr.io/chazmus/grandmaster-recall:latest
```

- **Web App & API**: Open [http://localhost:3001](http://localhost:3001) in your browser.
- **Data Persistence**: Your SQLite puzzle history and review schedules will be saved in `./data/chess_trainer.db`.

---

### Option 2: Run from Source (Development Mode)

Prerequisites: Rust (`cargo`), Node.js (`npm`), and `curl`.

```bash
./start.sh
```

- Automatically downloads the appropriate Stockfish 17+ binary for your architecture (`arm64` or `x86_64`).
- **Frontend (Vite HMR)**: [http://localhost:5173](http://localhost:5173)
- **Backend API**: [http://localhost:3001/api](http://localhost:3001/api)

---

## 🧪 Comprehensive Test Suite

Run the full end-to-end test suite (Rust unit/engine tests + React Vitest tests) with a single command:

```bash
./test.sh
```

### Individual Test Runners:
- **Rust Backend**:
  ```bash
  cd backend && cargo test
  ```
- **React Frontend**:
  ```bash
  cd frontend && npm test
  ```

---

## 📁 Architecture Overview

```
chess-trainer/
├── engine/
│   └── stockfish/           # Native Stockfish 17+ UCI binary
├── backend/                 # Rust Axum Web Service
│   ├── src/
│   │   ├── main.rs          # Server entry & Axum router
│   │   ├── engine.rs        # Stockfish process pool & UCI manager
│   │   ├── chess_com.rs     # Chess.com public API client
│   │   ├── analyzer.rs      # PGN walking & blunder/puzzle detection
│   │   ├── srs.rs           # SM-2 spaced repetition scheduler
│   │   ├── db.rs            # SQLite connection pool & queries
│   │   └── models.rs        # Data structures & API DTOs
│   └── data/
│       └── chess_trainer.db # Embedded SQLite database
├── frontend/                # React 19 + TypeScript + Vite + Tailwind v4
│   ├── public/
│   │   ├── piece/           # Lichess piece SVGs
│   │   └── sound/           # Lichess chess sound effects
│   └── src/
│       ├── components/
│       │   ├── Chessboard.tsx     # Chessground board component
│       │   ├── PuzzleSolver.tsx   # Interactive solving & punishment playout
│       │   ├── ReviewQueue.tsx    # Daily spaced repetition queue
│       │   ├── GameSyncModal.tsx  # Chess.com game import & live progress
│       │   ├── StatsDashboard.tsx # Tactical & opening analytics
│       │   └── Navbar.tsx         # Navigation & account switcher
│       ├── test/                  # Vitest Component & Sound unit tests
│       └── App.tsx          # Root application state
├── test.sh                  # Comprehensive test runner script
└── start.sh                 # App launcher script
```

---

## 📄 License

This project is licensed under the **[MIT License](./LICENSE)** © 2026 Chaz Bailey.

---

## 📜 Third-Party Assets & Attribution

This project uses chess piece artwork, audio themes, and libraries from the **[Lichess.org (Lila)](https://github.com/lichess-org/lila)** open-source ecosystem, **Stockfish**, and independent artists.

Please see **[`ATTRIBUTION.md`](./ATTRIBUTION.md)** for the complete list of piece artists, sound creators, and open-source licenses (GPLv3, AGPLv3, Apache 2.0, CC-BY, CC-BY-SA, MIT).
