#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="${SRCROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
WORKSPACE_ROOT="$(cd "$ROOT_DIR/../.." && pwd)"
APP_CONTENTS_PATH="${1:-${TARGET_BUILD_DIR:-}${CONTENTS_FOLDER_PATH:+/$CONTENTS_FOLDER_PATH}}"
BUILD_CONFIGURATION="${2:-${CONFIGURATION:-Release}}"
CACHE_DIR="$ROOT_DIR/.swift-cache"
ORIGINAL_HOME="${HOME:-}"

if [[ -z "$APP_CONTENTS_PATH" ]]; then
  printf 'usage: %s <app-bundle-path> [configuration]\n' "$0" >&2
  exit 1
fi

case "$BUILD_CONFIGURATION" in
  [Dd]ebug)
    RUST_PROFILE="debug"
    CARGO_ARGS=()
    ;;
  *)
    RUST_PROFILE="release"
    CARGO_ARGS=(--release)
    ;;
esac

mkdir -p "$CACHE_DIR/clang" "$CACHE_DIR/swiftpm" "$CACHE_DIR/home"

if [[ -n "$ORIGINAL_HOME" ]]; then
  export RUSTUP_HOME="${RUSTUP_HOME:-$ORIGINAL_HOME/.rustup}"
  export CARGO_HOME="${CARGO_HOME:-$ORIGINAL_HOME/.cargo}"
fi

export HOME="$CACHE_DIR/home"
export SWIFTPM_MODULECACHE_OVERRIDE="$CACHE_DIR/swiftpm"
export CLANG_MODULE_CACHE_PATH="$CACHE_DIR/clang"

if [[ -n "${CARGO_HOME:-}" ]]; then
  export PATH="$CARGO_HOME/bin:$PATH"
elif [[ -n "$ORIGINAL_HOME" && -d "$ORIGINAL_HOME/.cargo/bin" ]]; then
  export PATH="$ORIGINAL_HOME/.cargo/bin:$PATH"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo executable not found; install Rust or configure CARGO_HOME before building AgentRelay" >&2
  exit 1
fi

cd "$WORKSPACE_ROOT"
cargo build -p agrelay-cli --bin agrelay "${CARGO_ARGS[@]}"

RELAY_BINARY_PATH="$WORKSPACE_ROOT/target/$RUST_PROFILE/agrelay"
DESTINATION_DIR="$APP_CONTENTS_PATH/Helpers"
DESTINATION_PATH="$DESTINATION_DIR/agrelay"

if [[ ! -x "$RELAY_BINARY_PATH" ]]; then
  echo "expected agrelay binary not found at $RELAY_BINARY_PATH" >&2
  exit 1
fi

mkdir -p "$DESTINATION_DIR"
cp "$RELAY_BINARY_PATH" "$DESTINATION_PATH"
chmod +x "$DESTINATION_PATH"

SIGN_IDENTITY="-"
if [[ "${CODE_SIGNING_ALLOWED:-NO}" == "YES" && -n "${EXPANDED_CODE_SIGN_IDENTITY:-}" ]]; then
  SIGN_IDENTITY="$EXPANDED_CODE_SIGN_IDENTITY"
fi

codesign --force --sign "$SIGN_IDENTITY" --timestamp=none "$DESTINATION_PATH"
