# AgentRelay Engineering Architecture

## Purpose

This document describes the durable runtime boundaries, repository shape, and data flow for AgentRelay. It is a current architecture reference, not a project status board.

For the workspace layout and crate/module boundaries, see [Repository Shape](../AGENTS.md#repository-shape) in AGENTS.md.

## Layering Rules

1. `apps/*` can depend on `relay-core`.
2. `relay-core` stays modular inside one crate; prefer extending modules over adding new crates.
3. `relay-core::models` owns shared domain types, stable error codes, and JSON/RPC contracts.
4. `relay-core::services` owns use-case orchestration and business policy.
5. `relay-core::store` owns SQLite and file-backed persistence details.
6. `relay-core::platform` owns path resolution and platform-specific helpers.
7. `relay-core::adapters` owns provider-specific behavior only and must stay isolated from CLI parsing and Swift UI concerns.

## Runtime Boundaries

### CLI

The CLI is the only execution layer for profile management, switching, validation, usage refresh, diagnostics, and daemon hosting.

Its responsibilities are to:

- parse commands and JSON input
- render human-readable output and stable `--json` output
- convert failures into stable project error codes
- host the stdio JSON-RPC daemon transport used by native control planes

The command surface stays intentionally shallow:

- top-level commands such as `doctor`, `status`, `list`, `show`, `switch`, and `refresh`
- grouped commands such as `settings`, `autoswitch`, `activity`, and `codex`
- programmatic transport via `agrelay daemon --stdio`

### Daemon Session

`agrelay daemon --stdio` exposes a single-client stdio JSON-RPC 2.0 session.

- transport is newline-delimited UTF-8 JSON on `stdin` and `stdout`
- `stdout` is reserved for protocol messages
- logs and diagnostics go to `stderr`
- the daemon owns background refresh, auto-switch evaluation, switch execution, and state-change notifications
- the session is host-owned and long-lived; it is not a detached system service and does not support multiple concurrent clients

### Core Services

`relay-core::services` contains the main orchestration layer:

- `doctor_service` for environment checks and discovery
- `profile_service` for profile CRUD, validation, and summaries
- `status_service` for cached runtime state and health reporting
- `usage_service` for usage refresh, source selection, fallback behavior, and snapshot caching
- `switch_service` for transactional activation, validation, rollback, and checkpoint handling
- `policy_service` for auto-switch candidate selection and cooldown logic
- `diagnostics_service` for logs, exports, and redacted runtime snapshots

### Store And Platform

`relay-core::store` owns durable and cached state:

- SQLite stores profiles, settings, switch history, failure events, and linked provider identities
- SeaORM 2.x entity definitions are the schema source of truth
- file-backed caches store active state and usage snapshots for fast local reads
- snapshot directories hold rollback artifacts and operational exports

`relay-core::platform` resolves `AGRELAY_HOME`, default runtime paths, and platform-specific filesystem behavior. It also provides atomic-write and process-execution helpers.

### macOS Control Plane

The SwiftUI app in `apps/relay-macos` is a control plane over the daemon session.

- it launches and supervises `agrelay daemon --stdio`
- it sends JSON-RPC requests and subscribes to notifications
- it decodes stable protocol models shared from `relay-core::models`
- it must not duplicate switch logic or mutate live Codex files directly

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
- rollback checkpoints
- diagnostics exports

SQLite is the durable source of truth. File-backed state is limited to caches and operational artifacts that benefit from simple local reads.

## Service Flows

### Boot

1. resolve runtime paths from environment or the user home directory
2. ensure the AgentRelay home layout exists
3. open stores, reject incompatible legacy schemas, and run SeaORM schema sync for write mode
4. execute the requested use case

### Daemon Boot

1. a host program starts `agrelay daemon --stdio`
2. AgentRelay boots the same core stores and services used by synchronous CLI commands
3. the client sends `initialize` and optional subscription requests
4. the daemon returns initial state, performs startup refresh work, and begins interval-driven policy evaluation

### Profile Mutation

1. validate request input
2. persist the change in one SQLite transaction
3. update related caches only when the use case requires it
4. return a stable response envelope or human summary

### Switch Transaction

1. load the target profile and validate prerequisites
2. create a checkpoint of the managed live file set
3. write candidate files through temporary paths
4. atomically replace the managed live files
5. run post-switch validation
6. commit state on success or roll back and emit failure state on error

### Managed Live Files

AgentRelay does not replace the entire `~/.codex` directory during activation. It manages only the live file set required for Codex profile switching:

- `config.toml`
- `auth.json`
- `version.json`

This keeps live mutation recoverable and avoids copying unrelated history, logs, caches, or other session artifacts into the active home.

## Engineering Invariants

- every user-visible command supports `--json`
- programmatic integrations should prefer structured params or JSON input over ad hoc flag assembly
- user-visible failures map to stable `ErrorCode` values
- daemon RPC contracts in `relay-core::models` stay backward-compatible for the macOS control plane
- read-only flows should avoid unnecessary filesystem or database writes when practical
- no adapter mutates project-local `.codex/`
- live config writes are atomic and recoverable
- shared infrastructure stays agent-neutral where practical; provider-specific auth and usage semantics stay at the adapter edge

For the testing strategy and verification layers, see [Development](./development.md#test-strategy).
