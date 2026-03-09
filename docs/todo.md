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

## Alignment Backlog

These items capture the current gaps against the latest Relay product/function guide and should drive near-term work.

- [x] Add machine-friendly JSON input for mutation commands such as `profiles add`, `profiles edit`, `switch`, and `auto-switch`.
- [x] Define a stable JSON input contract that maps directly to request models and supports `--input-json <file>` plus stdin via `--input-json -`.
- [x] Reject ambiguous mixed input by defining clear precedence or mutual exclusion rules between flags and JSON payloads.
- [x] Add `relay usage` with a stable JSON response that covers session usage, weekly usage, reset time, freshness, and source.
- [x] Add usage domain models and storage for source confidence, stale state, and last refresh metadata.
- [x] Implement Codex usage collection with local-first behavior, local fallback behavior, and an explicit source label for degraded data.
- [x] Keep auto-switch decisions gated on high-confidence signals only, including session exhausted, weekly exhausted, auth invalid, rate limit, and quota exhausted.
- [x] Do not auto-switch on stale data or low-confidence usage estimates.
- [x] Exclude usage-exhausted or otherwise unhealthy profiles from `switch next` and auto-switch candidate selection.
- [x] Surface a stable exhausted-pool error when all enabled profiles are unavailable for auto-switch, and notify in the macOS control plane instead of retry-looping.
- [x] Fix switch synchronization so optional managed files do not leak across profiles when a target profile omits them.
- [x] Make rollback restore the exact pre-switch live file set, including cleanup of files introduced by a failed activation attempt.
- [x] Add regression tests for optional-file removal and rollback cleanup semantics during profile switches.
- [x] Reduce Codex-specific fields in shared models and status payloads so future agents can plug in through adapters with less cross-cutting schema churn.
- [x] Reconcile repository docs that still describe the macOS app as future-only with the current implemented app state.
- [x] Extend usage storage from a single active snapshot to per-profile cached snapshots so inactive profiles can be inspected without switching.
- [x] Add `relay usage profile`, `relay usage list`, `relay usage refresh <id>`, `relay usage refresh --enabled`, and `relay usage config set`.
- [x] Refresh inactive profile usage from profile-local state when available, and keep disabled profiles manual-only while enabled profiles participate in automatic refresh.
- [x] Add usage source mode settings (`Auto`, `Local`, `WebEnhanced`) and small menu-open debounce-based refresh behavior for the macOS control plane.
- [x] Add official Codex login import flow that creates an enabled profile and binds a dedicated probe identity record.
- [x] Add Codex relink flow for existing profiles so probe identity and managed auth can be refreshed without recreating the profile.
- [x] Add official remote usage probing for profiles with bound identities so non-current profiles can fetch true usage across the account pool.
- [x] Replace the Codex-specific probe identity table shape with a generic provider envelope plus provider-specific credentials/metadata payloads.
- [x] Remove remaining main-layer `codex_*` platform and diagnostics names so Codex-specific logic stays inside adapters and explicit agent selections.
- [x] Make bare `relay usage` a per-profile usage list, add `relay usage current`, and align the macOS menu/profile UI around usage badges and list-first presentation.
- [x] Replace pretty-JSON human CLI output with command-specific tables and summaries across all user-facing commands while keeping `--json` stable.

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
- [x] Implement `relay profiles import`.
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
