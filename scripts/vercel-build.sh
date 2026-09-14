#!/usr/bin/env bash
# Vercel build step: compile the web release and stage it as `dist/`.
#
# `dx` writes to a path that includes the crate name and the profile
# (target/dx/stroke/release/web/public). Copying it to a fixed directory keeps
# the Vercel output setting stable even if that layout changes between CLI
# versions.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
# shellcheck source=scripts/vercel-env.sh
source scripts/vercel-env.sh

log() { printf '\n\033[1m▸ %s\033[0m\n' "$*"; }

if ! command -v dx >/dev/null; then
  echo "dx is not on PATH — run scripts/vercel-install.sh first." >&2
  exit 1
fi

log "Building the web release with $(dx --version)"
dx build --release --platform web

target_dir="${CARGO_TARGET_DIR:-target}"
site="$(find "${target_dir}/dx" -type d -path '*/release/web/public' -print -quit 2>/dev/null || true)"
if [ -z "${site}" ]; then
  echo "Build finished but no web output under ${target_dir}/dx." >&2
  exit 1
fi

log "Staging ${site} as dist/"
rm -rf dist
mkdir -p dist
cp -R "${site}/." dist/

# A missing wasm file or a missing search index still produces a directory that
# looks plausible, and the failure only shows up as a blank page in production.
for required in dist/index.html dist/data/index.json; do
  [ -f "${required}" ] || { echo "Missing ${required} in the build output." >&2; exit 1; }
done
if ! find dist -name '*.wasm' -print -quit | grep -q .; then
  echo "No .wasm in the build output." >&2
  exit 1
fi

log "Done"
printf '  %s\n' \
  "$(du -sh dist | cut -f1) total" \
  "$(find dist -name '*.wasm' -printf '%f — %s bytes\n')" \
  "$(find dist/data/char -type f 2>/dev/null | wc -l) vendored character files"
