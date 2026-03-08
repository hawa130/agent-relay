# AGENTS.md

## Project Summary

Relay is a CLI-first local profile orchestrator for coding agents.

V1 scope:

- Core execution engine is a Rust CLI.
- Primary supported agent is `Codex`.
- Future macOS menu bar app is only a control plane over CLI JSON commands.

Primary references:

- [`docs/architecture.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/architecture.md)
- [`docs/todo.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/todo.md)
- [`relay_cli_first_macos_ui_v_1_dev_plan.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/relay_cli_first_macos_ui_v_1_dev_plan.md)

## Current Repository Shape

Keep the Rust project at two package levels only:

- `apps/relay-cli`: user-facing CLI entrypoint
- `crates/relay-core`: core library

Inside `relay-core`, organize code by modules instead of creating more crates:

- `models`: shared domain types, error codes, JSON protocol
- `services`: use-case orchestration
- `store`: SQLite and file-backed persistence
- `platform`: path detection and platform-specific helpers
- `adapters`: agent-specific behavior, currently Codex only

Do not reintroduce tiny crates for `types`, `store`, `platform`, or `adapters` unless there is a strong independent packaging boundary.

## Architecture Rules

- CLI is the only real execution layer.
- Any future macOS UI must call `relay` CLI commands and parse JSON output.
- UI code must not directly mutate Codex files.
- All user-visible commands must support `--json`.
- Errors exposed to callers must use stable project error codes.
- All live config mutations must be transactional and recoverable.
- Do not modify project-local `.codex/`; V1 only works on user-level Codex state.

## Product Constraints

V1 is intentionally limited:

- Support `Codex` first.
- Focus on local profile management, safe switching, rollback, and diagnostics.
- Do not build browser-cookie scraping, private API reverse engineering, or “quota bypass” logic.

## Implementation Status

Implemented now:

- workspace scaffold
- `relay` CLI command framework
- `relay doctor`
- `relay status`
- `relay profiles list/add/edit/remove/enable/disable/import-codex`
- `relay switch <id>`
- `relay switch next`
- `relay auto-switch enable/disable`
- `relay events list`
- `relay logs tail`
- `relay diagnostics export`
- SQLite-backed profile store
- file-backed active state cache
- switch checkpoints, rollback, switch history, and failure events
- CLI integration tests and core unit tests

Not implemented yet:

- macOS app

Check [`docs/todo.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/todo.md) before starting new work and update it when major milestones change.

## Working Commands

Common commands:

```bash
cargo fmt --all
cargo test
cargo run -p relay-cli --bin relay -- status --json
cargo run -p relay-cli --bin relay -- doctor --json
cargo run -p relay-cli --bin relay -- profiles list --json
cargo install --path apps/relay-cli
./scripts/release-local.sh
```

Use `RELAY_HOME` for isolated testing:

```bash
RELAY_HOME=/tmp/relay-smoke cargo run -p relay-cli --bin relay -- status --json
```

## Coding Guidance

- Prefer extending `relay-core` modules over adding new packages.
- Put business logic in `services`, not in the CLI entrypoint.
- Keep `models` stable because CLI JSON and future UI will depend on them.
- Put persistence details behind `store`.
- Keep agent-specific validation and activation behavior in `adapters`.
- Add tests for new store logic and service behavior.
- Use temp directories for tests touching filesystem state.

## Near-Term Priorities

Work in this order unless the task explicitly says otherwise:

1. SQLite migration/versioning policy
2. Linux support hardening
3. release packaging polish
4. macOS menu bar app
