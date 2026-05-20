# v1.3 nexus-server Dockerfile (Phase 1.3.9 — scaffold).
#
# Multi-stage build with cargo-chef for layer caching.
# Final image: distroless/cc-debian12 (~25 MB).
#
# Build:    docker buildx build -t nexus-server:dev .
# Run:      docker run --rm -p 50052:50052 -v ./certs:/etc/nexus:ro
#                      -e NEXUS_CA_CERT=/etc/nexus/ca.crt.pem [...]
#                      nexus-server:dev

# -----------------------------------------------------------------
# Stage 1: planner (compute the build plan via cargo-chef).
# -----------------------------------------------------------------
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------
# Stage 2: builder (cook deps from the plan; then build the binary).
# -----------------------------------------------------------------
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler libssl-dev pkg-config && \
    rm -rf /var/lib/apt/lists/*
RUN cargo chef cook --release --recipe-path recipe.json --bin nexus-server
COPY . .
RUN cargo build --release -p nexus-infra --bin nexus-server

# -----------------------------------------------------------------
# Stage 3: runtime (distroless/cc — minimal, no shell, no package manager).
# -----------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/nexus-server /usr/local/bin/nexus-server

# Distroless doesn't include a shell so we declare the user numerically.
USER 65532:65532
WORKDIR /var/lib/nexus

EXPOSE 50052 50051 9100
ENTRYPOINT ["/usr/local/bin/nexus-server"]
CMD ["--config", "/etc/nexus/nexus.toml"]
