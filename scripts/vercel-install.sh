#!/usr/bin/env bash
# Vercel install step: put a Rust toolchain and the Dioxus CLI on PATH.
#
# Vercel's build image is Amazon Linux 2023 with no Rust in it, so the whole
# toolchain is installed here rather than assumed. Everything lands inside the
# project directory, which is the part of the build container that is reliably
# writable.
#
# Safe to run anywhere, not just on Vercel: it skips whatever is already there.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
# shellcheck source=scripts/vercel-env.sh
source scripts/vercel-env.sh

log() { printf '\n\033[1m▸ %s\033[0m\n' "$*"; }

# ── C toolchain and OpenSSL ─────────────────────────────────────────────────
# Only needed to compile `dx` from source: it links against the system OpenSSL,
# and Amazon Linux 2023 ships the library but not its headers.
install_build_deps() {
  local dnf_pkgs=() apt_pkgs=()

  command -v cc >/dev/null || { dnf_pkgs+=(gcc); apt_pkgs+=(gcc); }
  command -v pkg-config >/dev/null || {
    dnf_pkgs+=(pkgconf-pkg-config)
    apt_pkgs+=(pkg-config)
  }
  [ -f /usr/include/openssl/ssl.h ] || {
    dnf_pkgs+=(openssl-devel)
    apt_pkgs+=(libssl-dev)
  }

  [ ${#dnf_pkgs[@]} -eq 0 ] && return 0

  log "Installing build dependencies: ${dnf_pkgs[*]}"
  if command -v dnf >/dev/null; then
    dnf install -y "${dnf_pkgs[@]}"
  elif command -v apt-get >/dev/null; then
    apt-get update -qq && apt-get install -y --no-install-recommends "${apt_pkgs[@]}"
  else
    echo "No dnf or apt-get. Install these yourself: ${dnf_pkgs[*]}" >&2
    return 1
  fi
}

# ── Rust ────────────────────────────────────────────────────────────────────
if command -v rustup >/dev/null; then
  log "Installing Rust ${RUST_VERSION} with the existing rustup"
  rustup toolchain install "${RUST_VERSION}" --profile minimal \
    --target wasm32-unknown-unknown
else
  log "Installing rustup and Rust ${RUST_VERSION}"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
    sh -s -- -y --no-modify-path --profile minimal \
      --default-toolchain "${RUST_VERSION}" --target wasm32-unknown-unknown
fi

rustc --version

# ── Dioxus CLI ──────────────────────────────────────────────────────────────
# `dx` is the linker for `asset!()`, so there is no building the site without it.
#
# Three ways to get it, cheapest first:
#
#   1. The copy an earlier build left in node_modules/.cache — the only
#      directory Vercel restores, which is why it lives in that odd place.
#   2. The prebuilt release binary. Instant, but it is built against glibc 2.39
#      and Amazon Linux 2023 has 2.34, so on Vercel it will not run. Hence the
#      probe: download it, ask it its version, believe only that.
#   3. Compiling it, which is the expensive path and the one Vercel takes.
dx_works() { [ -x "${DX_HOME}/bin/dx" ] && "${DX_HOME}/bin/dx" --version >/dev/null 2>&1; }

if dx_works; then
  log "Reusing cached $(dx --version)"
else
  rm -rf "${DX_HOME}"
  mkdir -p "${DX_HOME}/bin"

  tarball="dx-$(uname -m)-unknown-linux-gnu.tar.gz"
  url="https://github.com/DioxusLabs/dioxus/releases/download/v${DX_VERSION}/${tarball}"

  log "Trying the prebuilt dx ${DX_VERSION}"
  if command -v tar >/dev/null &&
    curl -fsSL "${url}" | tar xz -C "${DX_HOME}/bin" && dx_works; then
    echo "Prebuilt dx runs on this image."
  else
    echo "No usable prebuilt dx here — it wants tar and glibc 2.39."
    echo "Building it from source instead."
    rm -f "${DX_HOME}/bin/dx"
    install_build_deps

    log "Compiling dioxus-cli ${DX_VERSION} — this is the slow part"
    cargo install dioxus-cli \
      --version "${DX_VERSION}" \
      --locked \
      --features disable-telemetry \
      --root "${DX_HOME}"
    dx_works
  fi
fi

log "Install step done: $(dx --version)"
