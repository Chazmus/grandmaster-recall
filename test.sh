#!/usr/bin/env bash
set -e

export PATH="$HOME/.cargo/bin:$PATH"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=============================================="
echo "  Running Rust Backend Tests (Unit + Engine)  "
echo "=============================================="
cd "$PROJECT_DIR/backend"
cargo test

echo ""
echo "=============================================="
echo "  Running React Frontend Tests (Vitest)       "
echo "=============================================="
cd "$PROJECT_DIR/frontend"
npm test

echo ""
echo "=============================================="
echo "  ALL TEST SUITES PASSED SUCCESSFULLY!        "
echo "=============================================="
