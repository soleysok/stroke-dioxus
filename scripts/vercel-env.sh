#!/usr/bin/env bash
# Shared environment for the Vercel install and build steps.
#
# Vercel runs the install command and the build command as separate shells, so
# anything exported by one is gone by the time the other starts. Both source
# this file instead of duplicating the paths.
#
# Nothing here reads a value out of the surrounding environment. Names like
# RUST_VERSION and CARGO_HOME are commonly already set — by a base image, or by
# a machine's own Rust install — and inheriting one silently builds the site
# with the wrong toolchain. Bump the pins here in a commit instead.

# Rust 1.85 is the floor (see the README); this pins a known-good stable so a
# new release cannot break a deploy on its own. DX_VERSION must match the
# dioxus version in Cargo.toml.
export RUST_VERSION=1.90.0
export DX_VERSION=0.7.10

# Vercel restores `node_modules` between builds and nothing else, so the one
# artefact worth keeping — the `dx` binary, which costs around twenty minutes of
# CPU to compile — lives under it. The toolchain and the cargo registry are
# cheap enough to refetch and would crowd out the 1 GB cache limit.
export STROKE_CACHE_DIR="${PWD}/node_modules/.cache/stroke"
export DX_HOME="${STROKE_CACHE_DIR}/dx-${DX_VERSION}"

# Self-contained, inside the project directory: the one place in a build
# container that is reliably writable.
export RUSTUP_HOME="${PWD}/.rust/rustup"
export CARGO_HOME="${PWD}/.rust/cargo"
# Pins every cargo and rustc invocation, including the ones `dx` makes itself.
export RUSTUP_TOOLCHAIN="${RUST_VERSION}"

export PATH="${DX_HOME}/bin:${CARGO_HOME}/bin:${PATH}"

# A build server is not a useful data point for the Dioxus maintainers.
export DX_TELEMETRY_ENABLED=false
export CI=true
