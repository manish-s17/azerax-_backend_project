# ── Stage 1: Build ────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# SQLX_OFFLINE=true → uses .sqlx/ cache for compile-time SQL checking
# Run `cargo sqlx prepare` on your dev machine to generate .sqlx/
ENV SQLX_OFFLINE=true

# Cache dependencies layer
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null; true
RUN rm -rf src

# Copy real source + SQL cache
COPY src         ./src
COPY migrations  ./migrations
# .sqlx/ holds compile-time query metadata — commit this folder to git
COPY .sqlx       ./.sqlx

RUN touch src/main.rs
RUN cargo build --release

# ── Stage 2: Runtime ──────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 libpq5 wget \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/aezarx-backend ./aezarx-backend
COPY --from=builder /app/migrations ./migrations

RUN mkdir -p uploads/avatars manga-pages public/images

EXPOSE 4000

HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=5 \
  CMD wget -qO- http://localhost:4000/api/health | grep -q '"ok"' || exit 1

CMD ["./aezarx-backend"]
