#!/usr/bin/env bash
# Build the zomes to wasm and pack the DNA + hApp bundles.
#
# Run from the repository root. Uses nix develop when available so that the
# pinned rust-toolchain.toml and hc CLI from flake.lock are used; falls back to
# the locally installed toolchain otherwise.
set -euo pipefail

cd "$(dirname "$0")/.."

run() {
  if command -v nix >/dev/null 2>&1; then
    nix develop --no-update-lock-file --command "$@"
  else
    "$@"
  fi
}

# An empty RUSTUP_TOOLCHAIN makes rustup consult rust-toolchain.toml; without it
# this repo would silently build with the caller's default toolchain, which
# produces different wasm bytes (see README "Reproducible DNA").
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-}"

run cargo build --release --target wasm32-unknown-unknown
run hc dna pack crates/dnas/recovery/workdir
run hc app pack workdir --recursive

echo
echo "DNA:  crates/dnas/recovery/workdir/volla_recovery.dna"
echo "hApp: workdir/volla_recovery.happ"
