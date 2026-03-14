#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_PATH="$ROOT_DIR/AgentRelay.xcodeproj"

cd "$ROOT_DIR"
rm -rf "$PROJECT_PATH"
xcodegen generate --spec project.yml
