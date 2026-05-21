#!/usr/bin/env bash
# v1.4.x-2 — local multi-arch docker build verification.
#
# Cross-builds nexus-server + nexus-agent for linux/amd64 + linux/arm64
# under QEMU. Build-only (no push); the existing .github/workflows/docker.yml
# handles pushes on tag.
#
# Usage:    ./scripts/docker-multiarch-verify.sh
# Prereqs:  docker engine + buildx + the user in the docker group
#           (or run with sudo). QEMU binfmt_misc registration happens
#           automatically the first time the multi-arch builder is used.

set -euo pipefail

if ! docker info >/dev/null 2>&1; then
    echo "error: cannot reach the docker daemon." >&2
    echo "       check that docker is running and the current user is in the docker group" >&2
    echo "       (sudo usermod -aG docker \$USER && newgrp docker)" >&2
    exit 1
fi

BUILDER_NAME="${BUILDER_NAME:-nexus-multiarch}"
PLATFORMS="${PLATFORMS:-linux/amd64,linux/arm64}"

# Register QEMU emulators (idempotent — silently no-ops if already set up).
docker run --rm --privileged tonistiigi/binfmt:latest --install all >/dev/null

# Ensure a docker-container builder exists (the default `default` driver
# can't build for foreign platforms). Idempotent.
if ! docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
    docker buildx create --name "${BUILDER_NAME}" --driver docker-container --use
else
    docker buildx use "${BUILDER_NAME}"
fi
docker buildx inspect --bootstrap >/dev/null

build_image() {
    local file="$1"
    local image="$2"
    echo
    echo "==> building ${image} for ${PLATFORMS} from ${file}"
    docker buildx build \
        --file "${file}" \
        --platform "${PLATFORMS}" \
        --tag "${image}:multiarch-verify" \
        --progress plain \
        .
    echo "==> ${image} multi-arch build OK"
}

build_image "Dockerfile" "nexus-server"
build_image "Dockerfile.agent" "nexus-agent"

echo
echo "v1.4.x-2 multi-arch verification: PASS"
