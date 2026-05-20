# Docker quickstart

Two Dockerfiles at the repo root:

| File | Purpose | Final image base |
|---|---|---|
| `Dockerfile` | `nexus-server` | `gcr.io/distroless/cc-debian12` (target ~25 MB) |
| `Dockerfile.agent` | `nexus-agent` | `debian:stable-slim` (PTY runtime support) |

Both Dockerfiles use a multi-stage build with [cargo-chef](https://github.com/LukeMathWalker/cargo-chef)
for layer-cache friendliness across rebuilds.

## Prerequisites

- Docker 20.10+ with buildkit (Docker 23+ is default-buildkit)
- ~6 GiB of free disk space for the build cache; the final image is small but the intermediate compiler artifacts are large

## Build

```bash
# Build the server image.
docker buildx build -t nexus-server:dev .

# Build the agent image.
docker buildx build -t nexus-agent:dev -f Dockerfile.agent .
```

Both Dockerfiles pin the toolchain to `FROM rust:1-bookworm` — using
the rolling `1` tag so `cargo-chef` (whose transitive deps require
recent rustc) installs cleanly. If you want reproducible builds, pin
to a concrete tag like `rust:1.91-bookworm`.

## Verification (v1.3 status)

- Dockerfile syntax: verified
- `cargo-chef` install stage: verified compiles cleanly against `rust:1-bookworm`
- Full multi-stage release build: deferred (requires substantial disk + RAM for the workspace compile)

For a thin smoke-test of the syntax without running the full build:

```bash
# Build only up to the planner stage (just installs cargo-chef + computes
# the recipe; doesn't compile the workspace).
docker buildx build --target planner -t nexus-planner:dev .
```

## Quick local stack

```bash
# 1. Provision dev mTLS certs.
./scripts/gen-certs.sh

# 2. Bring up server + agent + Prometheus.
docker compose up --build

# 3. Confirm metrics are scrapeable.
curl localhost:9100/metrics | head

# 4. Confirm the agent registered.
docker compose logs nexus-server | grep "A2A agent registered"

# 5. Run the operator console against localhost:50052 (outside compose;
#    Tauri needs the host display).
```

`docker-compose.yml` brings up three services:

| Service | Purpose | Ports |
|---|---|---|
| `nexus-server` | C2 server | 50052 (A2A), 9100 (Prometheus) |
| `nexus-agent` | One example agent | — |
| `prometheus` | Pre-configured scraper of `nexus-server:9100` | 9090 |

The Prometheus dashboard lives at `http://localhost:9090`. From there,
query e.g. `nexus_a2a_active_agent_sessions` to confirm v1.3 metrics
flow.

## Production builds

Use `docker buildx build --platform linux/amd64 -t registry.example.com/nexus-server:vN.M.K .`
and push to your registry. The k8s overlays
([`deploy/k8s/overlays/prod/`](../../deploy/k8s/overlays/prod/))
reference image tags by digest — see [`kubernetes.md`](kubernetes.md).

### Multi-arch (v1.4)

v1.4 ships **amd64 + arm64** images for both `nexus-server` and
`nexus-agent`. The build is driven by
[`.github/workflows/docker.yml`](../../.github/workflows/docker.yml)
on tag-push, using `docker buildx` with `--platform
linux/amd64,linux/arm64` and QEMU emulation for the non-native arch.

To build locally:

```bash
# Initialize buildx (one-time).
docker buildx create --use --name nexus-buildx
docker buildx inspect --bootstrap

# Multi-arch build, no push (-load can't carry multi-arch).
docker buildx build --platform linux/amd64,linux/arm64 -t nexus-server:dev .

# Or push to a registry (`--push` is mutually exclusive with `--load`).
docker buildx build --platform linux/amd64,linux/arm64 \
    -t registry.example.com/nexus-server:v1.4.0 --push .
```

Native arm64 CI runners cut emulation overhead substantially; if the
GitHub plan supports them, swap `runs-on: ubuntu-latest` for
`runs-on: ubuntu-24.04-arm` in
[`docker.yml`](../../.github/workflows/docker.yml) for arm64 jobs.

## Hardening notes

- The server image runs as UID `65532:65532` (distroless's `nonroot`
  user); no shell, no package manager.
- Persistent data (`/var/lib/nexus`) is volume-mounted; back it up
  separately (see [`operations.md`](operations.md#backup--disaster-recovery)).
- `NEXUS_*_CERT` env vars carry sensitive material; in compose use
  `secrets:` references; in k8s use the `nexus-tls` Secret
  ([`secret-tls.yaml`](../../deploy/k8s/base/secret-tls.yaml)).
- The metrics port (9100) is plaintext. In compose this is bound to
  `localhost` only; in k8s it's a ClusterIP — operators expose it
  through a separate scrape-job ingress with whatever auth their
  Prometheus stack uses.
