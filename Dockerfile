# syntax=docker/dockerfile:1.7
# ─────────────────────────────────────────────────────────────────────────────
# Build the stock-signal Tauri desktop app for Linux x86_64 inside a container.
# Produces native installers (.deb, .AppImage) under /artifacts/bundle/. The
# image does NOT run the GUI — Tauri apps need a real desktop session.
#
#   Build the image:
#     docker build -t stock-signal-build .
#
#   Extract installers to ./out:
#     docker buildx build --target=artifacts --output=./out .
#     # or, if not using buildx:
#     id=$(docker create stock-signal-build) && \
#       docker cp "$id":/artifacts ./out && docker rm "$id"
# ─────────────────────────────────────────────────────────────────────────────

# ─── Stage 1: builder ────────────────────────────────────────────────────────
FROM rust:1.83-bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive
ENV NODE_MAJOR=20

# Tauri 2 system deps + librdkafka (cmake) + node 20
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        cmake \
        curl \
        file \
        libayatana-appindicator3-dev \
        libgtk-3-dev \
        libjavascriptcoregtk-4.1-dev \
        librsvg2-dev \
        libsasl2-dev \
        libsoup-3.0-dev \
        libssl-dev \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        pkg-config \
        wget \
    && curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install tauri-cli --version "^2" --locked

WORKDIR /app

# Frontend deps (cached when package*.json unchanged)
COPY package.json package-lock.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm ci

COPY . .

# Build frontend + Rust binary + bundles. Cache the cargo registry and
# target dir across builds so iterative rebuilds aren't an hour each.
# Artifacts are copied OUT of the cache-mounted target dir into the image
# layer at /artifacts before the RUN exits.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=stock-signal-cargo-reg \
    --mount=type=cache,target=/usr/local/cargo/git,id=stock-signal-cargo-git \
    --mount=type=cache,target=/app/src-tauri/target,id=stock-signal-target \
    cargo tauri build --bundles deb,appimage \
 && mkdir -p /artifacts \
 && cp -r src-tauri/target/release/bundle /artifacts/bundle \
 && cp src-tauri/target/release/stock-signal /artifacts/stock-signal

# ─── Stage 2: artifacts-only (extract with --target=artifacts) ───────────────
FROM scratch AS artifacts
COPY --from=builder /artifacts /

# ─── Stage 3: default image — slim layer holding the build outputs ───────────
FROM debian:bookworm-slim
WORKDIR /artifacts
COPY --from=builder /artifacts /artifacts
CMD ["sh", "-c", "find /artifacts -maxdepth 3 -type f | sort"]
