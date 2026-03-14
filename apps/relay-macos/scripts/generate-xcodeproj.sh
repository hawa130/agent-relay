#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_PATH="$ROOT_DIR/AgentRelay.xcodeproj"
PACKAGE_RESOLVED_SOURCE="$ROOT_DIR/Package.resolved"
PACKAGE_RESOLVED_DEST="$PROJECT_PATH/project.xcworkspace/xcshareddata/swiftpm/Package.resolved"

cd "$ROOT_DIR"
rm -rf "$PROJECT_PATH"
xcodegen generate --spec project.yml

mkdir -p "$(dirname "$PACKAGE_RESOLVED_DEST")"
cp "$PACKAGE_RESOLVED_SOURCE" "$PACKAGE_RESOLVED_DEST"
