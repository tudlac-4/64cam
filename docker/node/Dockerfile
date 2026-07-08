# ── Stage 1: cross-compile the node binary ───────────────────────────────────
# Always runs on the BUILD host (native speed); cross-linker handles the rest.
FROM --platform=$BUILDPLATFORM rust:1.85-slim-bookworm AS builder
WORKDIR /build

ARG TARGETPLATFORM

RUN case "$TARGETPLATFORM" in \
      "linux/arm64") \
        apt-get update && \
        apt-get install -y --no-install-recommends gcc-aarch64-linux-gnu && \
        rm -rf /var/lib/apt/lists/* && \
        rustup target add aarch64-unknown-linux-gnu ;; \
      *) \
        : ;; \
    esac

COPY . .

RUN case "$TARGETPLATFORM" in \
      "linux/arm64") \
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        cargo build --release -p node \
                    --target aarch64-unknown-linux-gnu && \
        cp target/aarch64-unknown-linux-gnu/release/node /node ;; \
      *) \
        cargo build --release -p node && \
        cp target/release/node /node ;; \
    esac

# ── Stage 2: fetch the correct MediaMTX binary for TARGETPLATFORM ────────────
# Runs on the build host (native curl); downloads the arch-specific tarball.
FROM --platform=$BUILDPLATFORM debian:bookworm-slim AS mediamtx-fetch
ARG MEDIAMTX_VERSION=v1.9.1
ARG TARGETPLATFORM

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

RUN case "$TARGETPLATFORM" in \
      "linux/arm64") MTX_ARCH=arm64 ;; \
      *)             MTX_ARCH=amd64 ;; \
    esac && \
    curl -fsSL \
      "https://github.com/bluenviron/mediamtx/releases/download/${MEDIAMTX_VERSION}/mediamtx_${MEDIAMTX_VERSION}_linux_${MTX_ARCH}.tar.gz" \
      | tar xz -C /usr/local/bin mediamtx && \
    chmod +x /usr/local/bin/mediamtx

# ── Stage 3: minimal runtime image ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=mediamtx-fetch /usr/local/bin/mediamtx /usr/local/bin/mediamtx
COPY --from=builder        /node                   /usr/local/bin/node

ENV MEDIAMTX_BIN=/usr/local/bin/mediamtx
ENV DATA_DIR=/data
VOLUME ["/data"]

# RTSP ingest
EXPOSE 8554
# WebRTC WHIP/WHEP signaling + ICE/RTP
EXPOSE 8889
# Playback HTTP server (segment serving + clip export)
EXPOSE 8890

CMD ["node"]
