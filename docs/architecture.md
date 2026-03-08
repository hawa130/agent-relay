# Relay V1 Engineering Architecture

## Goals

- Keep the CLI as the only execution engine for profile management, switching, validation, and diagnostics.
- Design the codebase so future Agent adapters can be added without rewriting storage, protocol, or UI integration.
- Make every state mutation explicit, testable, and recoverable.

## Monorepo Layout

```text
apps/
  relay-cli/        # User-facing CLI entrypoint
  relay-macos/      # Future native menu bar app; CLI client only
crates/
  relay-core/       # Core library with modules for models, services, store, adapters, platform
docs/
  architecture.md
  todo.md
```

## Layering Rules

1. `apps/*` can depend on `relay-core`.
2. `relay-core` is split by modules, not by tiny crates.
3. `relay-core::adapters` contains agent-specific behavior only. It must not know about CLI parsing or UI concerns.
4. `relay-core::store` owns persistence details. It should expose repository-style interfaces and avoid business policy.
5. `relay-core::models` contains shared contracts, error codes, and JSON protocol types.

## Runtime Boundaries

### CLI

- Parse commands and flags with `clap`.
- Produce either human-readable output or stable JSON envelopes.
- Convert internal errors into stable error codes for UI and scripts.

### Core Services

- `doctor_service`: environment checks, Codex discovery, config-path inspection.
- `profile_service`: CRUD, validation, import flow, enable/disable.
- `status_service`: read current cached state and summarize runtime health.
- `usage_service`: collect local-first Codex usage, apply fallback logic, and cache usage snapshots.
- `switch_service`: transactional activation, validation, rollback.
- `policy_service`: next-profile selection, cooldown, auto-switch policy.
- `diagnostics_service`: export logs, environment, redacted state.

### Store

- SQLite for durable relational data: profiles, settings, event history.
- JSON files for cached active state, usage snapshots, and low-latency UI reads.
- Snapshot directory for switch checkpoints and rollback assets.

### Platform

- Resolve `RELAY_HOME`, `~/.relay`, `~/.codex`, and agent-related paths.
- Provide filesystem helpers for atomic writes and process execution.
- Keep macOS/Linux-specific code isolated behind small modules.

## Package Strategy

- Keep the repo at two Rust package levels for V1: `relay-cli` and `relay-core`.
- Prefer directory modules inside `relay-core` over creating new crates.
- Only split out another crate when there is a real independent release/test boundary, not just a conceptual boundary.

## Data Model

### Profiles

- `Profile`: durable account/profile record.
- `AuthMode`: describes whether the profile is filesystem-backed, environment-backed, or keychain-backed.
- `AgentKind`: currently only `Codex`, but modeled as an enum for expansion.

### Active State

- Current active profile id.
- Last switch timestamp and result.
- Auto-switch enabled flag.
- Last known validation state.

### Events

- Failure reason.
- Trigger source: manual, health-check, command failure.
- Cooldown metadata and timestamps.

## Service Flow

### Boot

1. Resolve paths from env or home directory.
2. Ensure Relay home layout exists.
3. Open stores.
4. Run requested use-case.

### Profile Mutation

1. Validate user input.
2. Persist to SQLite in a single transaction.
3. Return a stable response envelope.

### Switch Transaction

1. Load target profile and validate static prerequisites.
2. Create checkpoint and backup current live config.
3. Write candidate config to temp files.
4. Atomically replace live config.
5. Run post-switch validation.
6. Commit state on success or rollback on failure.

## Engineering Conventions

- Every external command supports `--json`.
- Every user-visible failure maps to a stable `ErrorCode`.
- No adapter is allowed to mutate project-local `.codex/`.
- File writes that affect live agent config must be atomic and recoverable.
- Tests should prefer temp directories and deterministic fixtures.

## Testing Strategy

- `relay-core::models`: serde round-trip and protocol tests.
- `relay-core::store`: SQLite repository tests and atomic state-file tests.
- `relay-core::services`: service-level tests with temp stores and fake adapters.
- `relay-cli`: command parsing and JSON output smoke tests.
- Future: integration tests covering switch rollback against fixture directories.

## Release Strategy

- First milestone: a usable CLI with `doctor`, `status`, and profile CRUD.
- Second milestone: transactional switching and rollback.
- Third milestone: auto-switch and diagnostics.
- Fourth milestone: macOS menu bar app built strictly on top of CLI JSON.
