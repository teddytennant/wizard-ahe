#!/usr/bin/env bash
# Build the Nix task base image and load it into the local docker store.
# streamLayeredImage builds an executable that streams the image tarball to
# stdout, so no intermediate file is written. Idempotent: re-running simply
# rebuilds (cached) and re-loads the same tag.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "[build-task-image] nix build .#taskImage" >&2
nix build .#taskImage --out-link "$repo_root/result-task-image"

echo "[build-task-image] streaming image into docker load" >&2
"$repo_root/result-task-image" | docker load

echo "[build-task-image] loaded: wizard-ahe/task-base:latest"
docker image inspect wizard-ahe/task-base:latest --format 'image id: {{.Id}}  size: {{.Size}} bytes'
