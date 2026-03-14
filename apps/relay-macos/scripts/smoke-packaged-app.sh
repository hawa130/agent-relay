#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_ROOT="$(cd "$ROOT_DIR/../.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist/AgentRelay.app"
APP_EXECUTABLE="$APP_BUNDLE/Contents/MacOS/AgentRelay"
LOG_PREDICATE='process == "AgentRelay" OR eventMessage CONTAINS[c] "Library Validation failed" OR eventMessage CONTAINS[c] "Library not loaded: @rpath/AgentRelayUI.framework"'

cleanup() {
  pkill -x AgentRelay >/dev/null 2>&1 || true
  pkill -f "agrelay daemon --stdio" >/dev/null 2>&1 || true
}

cleanup
trap cleanup EXIT

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  "$ROOT_DIR/scripts/build-app.sh" >/dev/null
fi

if [[ ! -x "$APP_EXECUTABLE" ]]; then
  printf 'expected executable not found at %s\n' "$APP_EXECUTABLE" >&2
  exit 1
fi

START_TIME="$(python3 - <<'PY'
from datetime import datetime, timezone
print(datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S'))
PY
)"

open -n "$APP_BUNDLE"
sleep 2

sleep 1

if ! pgrep -x AgentRelay >/dev/null 2>&1; then
  /usr/bin/log show --start "$START_TIME" --predicate "$LOG_PREDICATE"
  printf 'packaged app did not remain running after launch\n' >&2
  exit 1
fi

if /usr/bin/log show --start "$START_TIME" --predicate "$LOG_PREDICATE" | /usr/bin/grep -Eq 'Library Validation failed|Library not loaded: @rpath/AgentRelayUI.framework'; then
  printf 'packaged app launch hit a runtime loading failure\n' >&2
  exit 1
fi

printf 'packaged app launch smoke check passed\n'
