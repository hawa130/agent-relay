# AgentRelay Engineering Architecture

## Goals

- Keep the CLI as the only execution engine for profile management, switching, validation, usage refresh, and diagnostics.
- Support additional agent providers without rewriting storage, protocol, or UI integration boundaries.
- Make every live-state mutation explicit, testable, and recoverable.

## Repository Layout

```text
apps/
  relay-cli/        # User-facing CLI entrypoint
  relay-macos/      # Native macOS control plane that supervises relay daemon sessions
crates/
  relay-core/       # Core library with models, services, store, adapters, and platform modules
docs/
  architecture.md
  install.md
  development.md
```

## Layering Rules

1. `apps/*` can depend on `relay-core`.
2. `relay-core` stays modular inside one crate; do not split it into tiny crates without a real packaging boundary.
3. `relay-core::adapters` owns provider-specific behavior only. It must not know about CLI parsing or Swift UI concerns.
4. `relay-core::store` owns persistence details and exposes repository-style interfaces, not business policy.
5. `relay-core::models` owns shared contracts, stable error codes, and JSON protocol types.

## Runtime Boundaries

### CLI

- Parse commands and JSON input with `clap` and request models.
- Produce human-readable output and stable `--json` output.
- Convert internal failures into stable project error codes.
- Host the stdio JSON-RPC daemon transport used by native control planes.

The command surface is intentionally shallow:

- top-level runtime commands: `doctor`, `status`, `list`, `show`, `edit`, `remove`, `enable`, `disable`, `switch`, `refresh`
- grouped commands: `settings`, `autoswitch`, `activity`, `codex`
- programmatic daemon transport: `agrelay daemon --stdio`

### Daemon Session

- `agrelay daemon --stdio` exposes a single-client stdio JSON-RPC 2.0 session.
- The daemon owns background refresh, auto-switch evaluation, switch execution, and state-change notifications.
- The transport is newline-delimited UTF-8 JSON on `stdin` and `stdout`.
- `stdout` is reserved for protocol messages only. Logs and diagnostics go to `stderr`.
- The current model is single-session and host-owned. It is not a detached system service and does not support multiple concurrent clients.

### Core Services

- `doctor_service`: environment checks, adapter discovery, config-path inspection
- `profile_service`: CRUD, validation, enable/disable, profile summaries
- `status_service`: current cached state and runtime health summaries
- `usage_service`: usage refresh, source selection, fallback behavior, snapshot caching
- `switch_service`: transactional activation, validation, rollback, checkpoint handling
- `policy_service`: next-profile selection, cooldown, autoswitch policy
- `diagnostics_service`: logs, exports, redacted environment snapshots

### Store

- SQLite stores durable relational state such as profiles, settings, switch history, failure events, and linked provider identities.
- `relay-core::store` owns hand-written SeaORM entities and uses SeaORM 2.x schema sync during write bootstrap.
- The entities are the schema source of truth; AgentRelay no longer treats versioned migrations as the primary workflow.
- File-backed caches store active state and usage snapshots for low-latency reads and reduced migration overhead.
- Snapshot directories store rollback assets for switch transactions.

### Platform

- Resolve `AGRELAY_HOME`, default runtime paths, and platform-specific filesystem locations.
- Provide atomic-write and process-execution helpers.
- Keep macOS/Linux-specific details isolated behind small modules.

### macOS Control Plane

- The SwiftUI app is a long-lived menu bar host that starts and supervises `agrelay daemon --stdio`.
- It sends JSON-RPC requests, subscribes to daemon notifications, and decodes stable protocol models from `relay-core::models`.
- It is a control plane only. It must not directly mutate Codex files or duplicate switch logic.

## State Model

### Durable SQLite State

- profiles
- app settings
- switch history
- failure events
- provider-linked probe identities

### File-Backed Runtime State

- active profile cache
- usage snapshot cache
- rollback checkpoints and exports

Use SQLite for durable truth and keep file-backed state limited to caches or operational artifacts that benefit from simple local reads.

## Service Flows

### Boot

1. Resolve paths from environment or home directory.
2. Ensure the AgentRelay home layout exists.
3. Open stores, reject incompatible legacy schemas, and run SeaORM schema sync for write mode.
4. Execute the requested use-case.

### Daemon Boot

1. The host program starts `agrelay daemon --stdio`.
2. AgentRelay boots the same core stores and services as synchronous CLI commands.
3. The client sends `initialize` and optional subscription requests.
4. The daemon returns initial state, performs startup refresh work, and begins interval-driven policy evaluation.

### Profile Mutation

1. Validate request input.
2. Persist the change in one SQLite transaction.
3. Update related caches only when required by the use-case.
4. Return a stable response envelope or human summary.

### Switch Transaction

1. Load the target profile and validate prerequisites.
2. Create a checkpoint of the live managed file set.
3. Write candidate files through temp paths.
4. Atomically replace the live managed files.
5. Run post-switch validation.
6. Commit state on success or rollback and emit failure state on error.

## Engineering Conventions

- Every user-visible command supports `--json`.
- Parameterized integrations should prefer JSON request payloads instead of ad hoc flag assembly.
- Every user-visible failure maps to a stable `ErrorCode`.
- Daemon RPC contracts live in `relay-core::models` and must remain backward-compatible for the macOS control plane.
- No adapter is allowed to mutate project-local `.codex/`.
- File writes that affect live agent config must be atomic and recoverable.
- Shared infrastructure should stay agent-neutral where practical; provider-specific auth and usage semantics belong in adapters.
- Tests should use temp directories and deterministic fixtures.

## Testing Strategy

- `relay-core::models`: serde round-trip and protocol tests
- `relay-core::store`: SeaORM migration/repository tests and state-file tests
- `relay-core::services`: service-level tests with temp stores and fake adapters
- `relay-cli`: command parsing, JSON contract, and integration smoke tests
- `relay-macos`: Swift decoding, daemon client, and supervisor integration tests
