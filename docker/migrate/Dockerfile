FROM --platform=$BUILDPLATFORM rust:1.88-slim-bookworm AS builder
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
        cargo build --release -p coordinator --bin migrate \
                    --target aarch64-unknown-linux-gnu && \
        cp target/aarch64-unknown-linux-gnu/release/migrate /migrate ;; \
      *) \
        cargo build --release -p coordinator --bin migrate && \
        cp target/release/migrate /migrate ;; \
    esac

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /migrate /usr/local/bin/migrate
CMD ["migrate"]
