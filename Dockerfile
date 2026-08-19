# ------------------------------------------------------------------------------
# Stage 1: Build React Frontend
# ------------------------------------------------------------------------------
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package*.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ------------------------------------------------------------------------------
# Stage 2: Build Rust Backend & Prepare Stockfish Engine
# ------------------------------------------------------------------------------
FROM rust:1.85-bookworm AS backend-builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Download Stockfish for the detected architecture
RUN set -eux; \
    mkdir -p /app/engine/stockfish; \
    ARCH=$(dpkg --print-architecture); \
    if [ "$ARCH" = "arm64" ]; then \
        SF_ARCH="arm64"; \
    else \
        SF_ARCH="x86-64"; \
    fi; \
    curl -fsSL "https://github.com/official-stockfish/Stockfish/releases/download/stockfish-dev-20260810-5062aee5/stockfish-linux-${SF_ARCH}-universal.tar.gz" -o /tmp/stockfish.tar.gz; \
    tar -xzf /tmp/stockfish.tar.gz -C /app/engine/stockfish --strip-components=1; \
    rm /tmp/stockfish.tar.gz; \
    chmod +x /app/engine/stockfish/stockfish-linux-*-universal

# Copy and build backend
COPY backend/Cargo.toml backend/Cargo.lock* ./backend/
WORKDIR /app/backend

# Dummy build for caching cargo dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src

COPY backend/src ./src
RUN touch src/main.rs && cargo build --release

# ------------------------------------------------------------------------------
# Stage 3: Runtime Image
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    zlib1g \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/backend/target/release/chess-trainer-backend /app/chess-trainer-backend

# Copy Stockfish binary
COPY --from=backend-builder /app/engine/stockfish /app/engine/stockfish

# Copy frontend static build
COPY --from=frontend-builder /app/frontend/dist /app/dist

# Setup persistent directory
RUN mkdir -p /app/data

ENV PORT=3001
ENV DATA_DIR=/app/data
ENV DIST_DIR=/app/dist
ENV RUST_LOG=info

EXPOSE 3001

CMD ["/app/chess-trainer-backend"]
