# syntax=docker/dockerfile:1
FROM rust:1.92-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config libsqlite3-dev && rm -rf /var/lib/apt/lists/*
COPY yt-dlp ./yt-dlp
COPY toobarr/Cargo.toml toobarr/Cargo.lock ./toobarr/
COPY toobarr/src ./toobarr/src
COPY toobarr/migrations ./toobarr/migrations
COPY toobarr/templates ./toobarr/templates
WORKDIR /build/toobarr
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/toobarr/target \
    cargo build --release && cp target/release/toobarr /toobarr

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ca-certificates libsqlite3-0 ffmpeg python3-pip curl unzip && \
    pip3 install --break-system-packages yt-dlp && \
    curl -fsSL https://deno.land/install.sh | DENO_INSTALL=/usr/local sh && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /toobarr .
COPY toobarr/templates ./templates
COPY toobarr/static ./static
COPY toobarr/migrations ./migrations
VOLUME ["/app/data", "/app/downloads"]
EXPOSE 8000
ENV PORT=8000
CMD ["./toobarr"]
