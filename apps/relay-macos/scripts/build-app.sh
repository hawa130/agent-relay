#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_ROOT="$(cd "$ROOT_DIR/../.." && pwd)"
BUILD_DIR="$ROOT_DIR/.build"
CACHE_DIR="$ROOT_DIR/.swift-cache"
ORIGINAL_HOME="${HOME:-}"
APP_NAME="AgentRelay"
CONFIGURATION="${CONFIGURATION:-release}"
PRODUCT_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$PRODUCT_DIR/${APP_NAME}.app"
XCODE_CONFIGURATION="Release"
if [[ "$CONFIGURATION" == "debug" ]]; then
  XCODE_CONFIGURATION="Debug"
fi
XCODE_BUILD_DIR="$ROOT_DIR/.xcode-build"
XCODE_PROJECT="$ROOT_DIR/AgentRelay.xcodeproj"
XCODE_DESTINATION="platform=macOS"
XCODE_PACKAGE_ARGS=(
  -clonedSourcePackagesDirPath "$BUILD_DIR"
  -disableAutomaticPackageResolution
  -onlyUsePackageVersionsFromResolvedFile
)
RUST_PROFILE="release"
CARGO_ARGS=(--release)
RELAY_BINARY_PATH="$WORKSPACE_ROOT/target/$RUST_PROFILE/agrelay"

if [[ "$CONFIGURATION" == "debug" ]]; then
  RUST_PROFILE="debug"
  CARGO_ARGS=()
  RELAY_BINARY_PATH="$WORKSPACE_ROOT/target/$RUST_PROFILE/agrelay"
fi

mkdir -p "$CACHE_DIR/clang" "$CACHE_DIR/swiftpm" "$CACHE_DIR/home" "$PRODUCT_DIR"

if [[ -n "$ORIGINAL_HOME" ]]; then
  export RUSTUP_HOME="${RUSTUP_HOME:-$ORIGINAL_HOME/.rustup}"
  export CARGO_HOME="${CARGO_HOME:-$ORIGINAL_HOME/.cargo}"
fi

export HOME="$CACHE_DIR/home"
export SWIFTPM_MODULECACHE_OVERRIDE="$CACHE_DIR/swiftpm"
export CLANG_MODULE_CACHE_PATH="$CACHE_DIR/clang"

cd "$ROOT_DIR"
./scripts/generate-xcodeproj.sh
xcodebuild \
  -project "$XCODE_PROJECT" \
  -scheme "$APP_NAME" \
  -configuration "$XCODE_CONFIGURATION" \
  -destination "$XCODE_DESTINATION" \
  -derivedDataPath "$XCODE_BUILD_DIR" \
  "${XCODE_PACKAGE_ARGS[@]}" \
  build
cargo build -p agrelay-cli --bin agrelay "${CARGO_ARGS[@]}"

XCODE_APP_BUNDLE="$XCODE_BUILD_DIR/Build/Products/${XCODE_CONFIGURATION}/${APP_NAME}.app"
EXECUTABLE_PATH="$XCODE_APP_BUNDLE/Contents/MacOS/${APP_NAME}"

if [[ ! -d "$XCODE_APP_BUNDLE" ]]; then
  echo "expected app bundle not found at $XCODE_APP_BUNDLE" >&2
  exit 1
fi

if [[ ! -x "$EXECUTABLE_PATH" ]]; then
  echo "expected executable not found at $EXECUTABLE_PATH" >&2
  exit 1
fi

if [[ ! -x "$RELAY_BINARY_PATH" ]]; then
  echo "expected agrelay binary not found at $RELAY_BINARY_PATH" >&2
  exit 1
fi

rm -rf "$APP_BUNDLE"
cp -R "$XCODE_APP_BUNDLE" "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/Resources/bin"
cp "$RELAY_BINARY_PATH" "$APP_BUNDLE/Contents/Resources/bin/agrelay"

chmod +x "$APP_BUNDLE/Contents/Resources/bin/agrelay"

codesign --force --sign - --timestamp=none "$APP_BUNDLE/Contents/Resources/bin/agrelay"
codesign --force --sign - --timestamp=none --deep "$APP_BUNDLE"

echo "built app bundle: $APP_BUNDLE"
