# syntax=docker/dockerfile:1.7

# ---------- 1. plan: 用 cargo-chef 提取依赖图 ----------
FROM rust:1-slim-bookworm AS chef
WORKDIR /app
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------- 2. builder: 先编依赖（可缓存），再编业务 ----------
FROM chef AS builder
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config \
 && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin zhiying-backend

# ---------- 3. runtime: distroless ----------
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/zhiying-backend /usr/local/bin/zhiying-backend
EXPOSE 9000
USER nonroot
ENTRYPOINT ["/usr/local/bin/zhiying-backend"]
