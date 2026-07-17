FROM rust:1.85-slim AS builder
WORKDIR /app

# Copy workspace root + lockfile
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members (Cargo needs them to resolve workspace)
COPY packages/core packages/core
COPY apps/cli apps/cli
COPY apps/desktop/src-tauri apps/desktop/src-tauri
COPY services/sync services/sync
COPY services/plugins services/plugins
COPY services/telemetry services/telemetry
COPY plugins/header-juggler plugins/header-juggler

# Build only the service we need
RUN cargo build --release --bin reqforge-plugins && \
    strip target/release/reqforge-plugins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/reqforge-plugins /usr/local/bin/
EXPOSE 3000
ENTRYPOINT ["reqforge-plugins"]
