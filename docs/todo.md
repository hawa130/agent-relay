# Relay V1 Todo

## Current Status

- `done`: monorepo and simplified package layout
- `done`: initial domain model and JSON response contract
- `done`: V1 CLI foundation and command surface
- `done`: real Codex switching transaction
- `done`: diagnostics, events, and auto-switch policy
- `done`: repository CI for `fmt`, `clippy`, and `test`
- `done`: SQLite schema versioning policy and automatic migration bootstrap
- `done`: native macOS control plane built on top of Relay CLI JSON

## Phase 0: Foundation

- [x] Create monorepo layout for apps, crates, and docs.
- [x] Create Rust workspace and shared dependency policy.
- [x] Collapse over-fragmented crates into a single `relay-core` library with modules.
- [x] Define stable JSON response envelope.
- [x] Define stable error code taxonomy.
- [x] Add baseline `.gitignore` and formatting config.
- [x] Expand repository CI from `fmt` + `test` to `fmt`, `clippy`, and `test`.
- [x] Add contributor setup document and local dev bootstrap script.
- [x] Expand the placeholder `apps/relay-macos` directory into a buildable skeleton with build instructions.

## Phase 1: Doctor + Profile Management

- [x] Implement Relay home path resolution with `RELAY_HOME` override.
- [x] Create SQLite-backed profile store.
- [x] Create file-backed active state cache.
- [x] Implement `relay doctor`.
- [x] Implement `relay status`.
- [x] Implement `relay profiles list`.
- [x] Implement `relay profiles add`.
- [x] Implement `relay profiles remove`.
- [x] Implement `relay profiles enable`.
- [x] Implement `relay profiles disable`.
- [x] Implement `relay profiles import-codex`.
- [x] Add profile edit/update command.
- [x] Add richer profile validation rules for Codex config directories.
- [x] Add integration tests that exercise the CLI end to end.

## Phase 2: Switching Transaction

- [x] Define `AgentAdapter` activation contract for switch transactions.
- [x] Implement Codex live-config discovery.
- [x] Implement checkpoint creation and backup metadata.
- [x] Implement atomic write helper with fsync + rename semantics.
- [x] Implement switch preflight validation.
- [x] Implement dynamic post-switch validation command.
- [x] Implement rollback on validation failure.
- [x] Implement `relay switch <id>`.
- [x] Implement `relay switch next`.
- [x] Persist switch history and active state updates.
- [x] Add failure injection tests for rollback correctness.

## Phase 3: Auto-Switch + Events

- [x] Create failure-event schema and persistence.
- [x] Define cooldown policy and next-profile selection strategy.
- [x] Implement `relay auto-switch enable`.
- [x] Implement `relay auto-switch disable`.
- [x] Implement `relay events list`.
- [x] Implement health-check driven failover hooks.
- [x] Add rate-limit/auth failure classification for Codex adapter.

## Phase 4: Diagnostics + Observability

- [x] Add `tracing` subscriber configuration and file logging.
- [x] Redact secrets and sensitive paths in logs.
- [x] Implement `relay logs tail`.
- [x] Implement `relay diagnostics export`.
- [x] Add machine-readable environment snapshot.
- [x] Add version/build metadata reporting.

## Phase 5: macOS App

- [x] Create native macOS target with `MenuBarExtra`.
- [x] Build a CLI client wrapper using `Process`.
- [x] Add status polling and JSON decoding layer.
- [x] Add menu bar profile list and manual switch action.
- [x] Add settings window for profiles and auto-switch.
- [x] Add activity/diagnostics views.
- [x] Add launch-at-login.
- [x] Add notification flows for switch success/failure.

## Cross-Cutting

- [x] Define semantic versioning and migration policy for SQLite schema.
- [x] Add `relay db migrate` or automatic migration bootstrap.
- [x] Add release packaging scripts.
- [x] Add Linux support matrix and platform test plan.
- [x] Add security review checklist for secrets and rollback safety.
- [x] Add user-facing documentation for install, upgrade, and recovery.
