# ── Stage 1: build the React frontend ───────────────────────────────────────
# node:22-alpine is a multi-arch manifest; always runs native (no QEMU needed).
FROM --platform=$BUILDPLATFORM node:22-alpine AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --prefer-offline
COPY frontend/ ./
RUN npm run build

# ── Stage 2: cross-compile the coordinator binary ────────────────────────────
# Always run on the BUILD host (native speed).
# When TARGETPLATFORM ≠ BUILDPLATFORM we install the cross linker and a second
# Rust target; the resulting binary is placed at /coordinator regardless of arch.
FROM --platform=$BUILDPLATFORM rust:1.82-slim-bookworm AS backend-builder
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
        cargo build --release -p coordinator --bin coordinator \
                    --target aarch64-unknown-linux-gnu && \
        cp target/aarch64-unknown-linux-gnu/release/coordinator /coordinator ;; \
      *) \
        cargo build --release -p coordinator --bin coordinator && \
        cp target/release/coordinator /coordinator ;; \
    esac

# ── Stage 3: minimal runtime image ───────────────────────────────────────────
# Base image is a multi-arch manifest; Docker pulls the correct variant
# matching TARGETPLATFORM automatically.
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder  /coordinator  /usr/local/bin/coordinator
COPY --from=frontend-builder /app/dist     /frontend

ENV FRONTEND_DIR=/frontend
EXPOSE 8080
CMD ["coordinator"]
