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

mkdir -p "$CACHE_DIR/clang" "$CACHE_DIR/swiftpm" "$CACHE_DIR/home" "$PRODUCT_DIR"
rm -rf "$XCODE_BUILD_DIR"

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
  ENABLE_HARDENED_RUNTIME=NO \
  build

XCODE_APP_BUNDLE="$XCODE_BUILD_DIR/Build/Products/${XCODE_CONFIGURATION}/${APP_NAME}.app"
EXECUTABLE_PATH="$XCODE_APP_BUNDLE/Contents/MacOS/${APP_NAME}"
BUNDLED_RELAY_PATH="$XCODE_APP_BUNDLE/Contents/Helpers/agrelay"

if [[ ! -d "$XCODE_APP_BUNDLE" ]]; then
  echo "expected app bundle not found at $XCODE_APP_BUNDLE" >&2
  exit 1
fi

if [[ ! -x "$EXECUTABLE_PATH" ]]; then
  echo "expected executable not found at $EXECUTABLE_PATH" >&2
  exit 1
fi

if [[ ! -x "$BUNDLED_RELAY_PATH" ]]; then
  echo "expected bundled agrelay not found at $BUNDLED_RELAY_PATH" >&2
  exit 1
fi

rm -rf "$APP_BUNDLE"
cp -R "$XCODE_APP_BUNDLE" "$APP_BUNDLE"

echo "built app bundle: $APP_BUNDLE"
