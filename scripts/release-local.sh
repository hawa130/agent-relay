#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"

mkdir -p "${DIST_DIR}"

cd "${ROOT_DIR}"

echo "[1/4] format check"
cargo fmt --all --check

echo "[2/4] test"
cargo test

echo "[3/4] release build"
cargo build --release -p relay-cli --bin relay

echo "[4/4] package binary"
cp "${ROOT_DIR}/target/release/relay" "${DIST_DIR}/relay"
chmod +x "${DIST_DIR}/relay"

echo "release artifact: ${DIST_DIR}/relay"
