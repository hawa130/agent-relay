# AGENTS.md

## Purpose

This file defines stable implementation constraints for contributors and coding agents.

Do not use it as a project status board, backlog dump, or feature snapshot. Keep time-sensitive state in code, tests, issues, or current task context instead.

## Canonical Docs

Use these documents as the long-lived sources of truth:

- [`README.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/README.md) for project overview, command surface, and quick start
- [`docs/architecture.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/architecture.md) for runtime boundaries and module responsibilities
- [`docs/install.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/install.md) for installation and operator usage
- [`docs/development.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/development.md) for contributor workflows and release checks
- [`docs/sqlite-schema.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/sqlite-schema.md) for SQLite schema workflow and bootstrap expectations
- [`docs/linux-support.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/linux-support.md) for Linux scope and test expectations
- [`docs/security-checklist.md`](/Users/hawa130/SoftwareProjects/relay-agent-switch/docs/security-checklist.md) for release-time security review

## Repository Shape

Keep the Rust project at two package levels only:

- `apps/relay-cli`: user-facing CLI entrypoint
- `crates/relay-core`: core library

The native macOS control plane lives in:

- `apps/relay-macos`

Inside `relay-core`, prefer module boundaries over new crates:

- `models`: shared domain types, error codes, JSON protocol
- `services`: use-case orchestration
- `store`: SQLite and file-backed persistence
- `platform`: path detection and platform-specific helpers
- `adapters`: agent-specific behavior, currently Codex only

Do not reintroduce tiny crates for `types`, `store`, `platform`, or `adapters` unless there is a strong packaging boundary that justifies it.

## Architecture Rules

- CLI is the only execution layer for profile management, switching, validation, and diagnostics.
- UI code must call `agrelay` CLI commands and parse JSON output; it must not mutate live agent files directly.
- All user-visible commands must support `--json`.
- Parameterized UI-to-CLI calls should prefer JSON input instead of rebuilding flag combinations ad hoc.
- Errors exposed to callers must use stable project error codes.
- Live config mutations must be transactional and recoverable.
- Read-only commands should avoid unnecessary filesystem or database writes when possible.
- Do not modify project-local `.codex/`; only user-level Codex state is in scope.
- Keep shared infrastructure agent-agnostic where practical. Provider-specific auth, usage, and file semantics belong at the adapter edge.
- Profile-linked identities and credentials must support provider-specific payload shapes. Do not keep adding nullable shared columns for provider-only fields.
- Shared interface names, diagnostics keys, and control-plane method names should stay agent-neutral unless the value itself is an explicit provider choice such as `codex`.

## Product Boundaries

AgentRelay is intentionally limited:

- Support `Codex` first.
- Focus on local profile management, safe switching, rollback, usage visibility, and diagnostics.
- Keep the macOS app as a control plane over the CLI, not a parallel execution path.
- Do not build browser-cookie scraping, private API reverse engineering, or quota-bypass logic.

## Working Rules

- Prefer extending `relay-core` modules over adding new packages.
- Run the repository formatter after code changes. Use `just fmt` to write formatting updates and `just fmt-check` for verification-oriented checks such as release/CI flows.
- Put business logic in `services`, not in the CLI entrypoint or Swift UI layer.
- Keep `models` stable because CLI JSON and the macOS app depend on them.
- Put persistence details behind `store`.
- Keep agent-specific validation, activation, login, and usage behavior in `adapters`.
- When adding provider-specific capabilities, first decide whether the code belongs in a reusable transport/provider utility or in the provider adapter. Default to the narrower boundary.
- Add tests for new store logic and service behavior.
- For UI/CLI protocol changes, add Swift-side decoding or client tests as needed.
- Use temp directories for tests that touch filesystem state.

## Commit Conventions

- Use Conventional Commits style prefixes such as `feat`, `fix`, `chore`, `docs`, `refactor`, and `test`.
- Start commit subjects with a lowercase letter.
- Keep commit subjects short, imperative, and scoped to the change actually being made.
- Prefer one coherent change per commit so history stays reviewable.

## Active Priorities

Unless a task says otherwise, bias new work toward these areas:

1. Linux support hardening
2. Release packaging polish
3. Usage confidence and source-policy hardening
4. Documentation consistency and operator clarity

## Documentation Maintenance

- Do not add dated status reports, completed-only todo lists, or phase plans as long-lived repo docs.
- If a planning document is temporary, either delete it after execution or fold its lasting guidance into the canonical docs above.
- Keep `AGENTS.md` focused on durable constraints and working rules, not implementation status.
- When command names, schema versions, or architecture boundaries change, update the canonical docs in the same change.
- Keep SQLite guidance aligned with the current SeaORM entity-first workflow; do not reintroduce stale references to versioned migration crates unless the architecture changes again.
