# Relay Technical Report

Date: 2026-03-09

## Summary

Relay is currently a CLI-first local profile orchestrator with a SwiftUI macOS control plane layered on top of the CLI JSON protocol. `Codex` is the only implemented adapter today.

The implementation has three main goals:

- keep Relay's own profile state as the orchestration truth
- safely synchronize active profile state into live agent files
- collect and display usage with clear source and confidence semantics

The codebase is split into:

- [`apps/relay-cli`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-cli)
- [`crates/relay-core`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core)
- [`apps/relay-macos`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos)

## Architecture

### CLI

The CLI is the only execution layer. It is responsible for:

- profile CRUD
- active profile switching
- rollback and switch history
- usage reads and refreshes
- account import and relink flows
- diagnostics and logs

All user-facing operations support `--json` output, and parameterized commands also support JSON input via `--input-json`.

Key entrypoint:

- [`main.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-cli/src/main.rs)

### Core Library

`relay-core` contains the domain logic and persistence layers.

Main module boundaries:

- `models`
- `services`
- `store`
- `platform`
- `adapters`

Important services:

- [`profile_service.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/services/profile_service.rs)
- [`switch_service.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/services/switch_service.rs)
- [`usage_service.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/services/usage_service.rs)
- Codex-specific login, auth, and usage logic now lives under [`adapters/codex`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/adapters/codex)

### macOS App

The SwiftUI app is a control plane only. It shells out to the `relay` binary, sends JSON on stdin where needed, and decodes JSON responses.

Important files:

- [`RelayCLIClient.swift`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos/RelayApp/Services/RelayCLIClient.swift)
- [`RelayAppModel.swift`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos/RelayApp/App/RelayAppModel.swift)
- [`SettingsView.swift`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos/RelayApp/Settings/SettingsView.swift)
- [`MenuBarView.swift`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos/RelayApp/MenuBar/MenuBarView.swift)

## Profile Model

The primary profile structure is defined in:

- [`profile.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/models/profile.rs)

Fields:

- `id`: Relay-managed stable identifier
- `nickname`: user-facing label
- `agent`: agent kind, currently `Codex`
- `priority`: ordering / selection weight
- `enabled`: whether the profile participates in switching and automatic refresh
- `agent_home`: profile-specific home directory
- `config_path`: profile-specific config path
- `auth_mode`: how auth/config should be interpreted
- `metadata`: reserved extension field
- `created_at` / `updated_at`

Why this shape:

- Relay-owned orchestration state lives on the profile object
- local filesystem state is referenced, not embedded
- future non-Codex agents can reuse the same shell without rewriting the base model
- volatile credentials and usage state are intentionally kept out of the main profile record

## Probe Identity Model

Remote account identity is stored separately from the main profile:

- [`probe_identity.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/models/probe_identity.rs)

Fields:

- `profile_id`
- `provider`
- `principal_id`
- `display_name`
- `credentials`
- `metadata`
- `created_at` / `updated_at`

Why this is separate:

- tokens refresh independently of profile config
- one profile can have stable local config while credentials rotate
- usage probing should not force the main profile record to become provider-specific
- provider-specific auth shapes can evolve without forcing a new shared table column for every provider

## Usage Model

Usage snapshots are defined in:

- [`usage.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/models/usage.rs)

Current usage semantics:

- source can be `Local`, `Fallback`, or `WebEnhanced`
- confidence is explicit
- stale state is explicit
- session and weekly windows are represented independently
- auto-switch eligibility is derived only from high-confidence, non-stale signals

Relay currently supports:

- active profile usage reads
- per-profile cached snapshots
- inactive profile refresh
- official remote probing when a profile has a probe identity

## Persistence

### SQLite

SQLite persistence is implemented in:

- [`profile_store.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/store/profile_store.rs)

Current schema version: `4`

Tables:

- `profiles`
- `app_settings`
- `switch_history`
- `failure_events`
- `profile_probe_identities`

#### `profiles`

Stores the core orchestrator state for each profile:

- `id`
- `nickname`
- `agent`
- `priority`
- `enabled`
- `agent_home`
- `config_path`
- `auth_mode`
- `metadata`
- `created_at`
- `updated_at`

#### `profile_probe_identities`

Stores per-profile remote account identity:

- `profile_id`
- `provider`
- `principal_id`
- `display_name`
- `credentials_json`
- `metadata_json`
- `created_at`
- `updated_at`

#### `switch_history`

Append-only operational history for profile switches.

#### `failure_events`

Recent failures used for cooldown and fallback status derivation.

#### `app_settings`

Key/value store for:

- auto-switch enablement
- cooldown
- usage source mode
- Codex agent settings for usage source mode

### File-backed State

Not all runtime state is in SQLite.

Current file-backed stores:

- active state cache
- usage snapshot cache

Key files:

- [`state_store.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/store/state_store.rs)
- [`usage_store.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/store/usage_store.rs)

Why file-backed caches still exist:

- simpler read/write path for lightweight state
- easier compatibility with read-only CLI bootstraps
- lower migration overhead for ephemeral data

Tradeoff:

- Relay does not yet have a single fully transactional SQLite truth for all runtime state

## Usage Collection Strategy

The usage pipeline lives in:

- [`usage_service.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/services/usage_service.rs)

Provider order is controlled by `UsageSourceMode`.

Current sources:

- `Local`
  - adapter-local RPC for the active profile when possible
  - adapter-local session parsing
- `WebEnhanced`
  - official remote usage endpoint for profiles with stored probe identity
- `Fallback`
  - failure events and cached snapshots

Important correction already made:

- inactive profiles no longer incorrectly fall back to the current live agent home when they lack their own `agent_home`

## Switching

Switching is performed via:

- [`switch_service.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/services/switch_service.rs)
- [`adapters/mod.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/adapters/mod.rs)

Current behavior:

- validate target profile
- create checkpoint
- back up managed live files
- atomically sync managed files
- validate live state after apply
- on failure, rollback
- record switch history and failure events

Managed live file set:

- `config.toml`
- optional `auth.json`
- optional `version.json`

## Current Add Account Flow

Current behavior after the latest fixes:

- UI remains a control plane
- UI calls CLI `profiles login codex`
- CLI/core directly runs `codex login`
- on successful login, Relay creates a new profile snapshot and imports the resulting auth state
- default nickname is derived from the logged-in account email when available

Related files:

- [`login.rs`](/Users/hawa130/SoftwareProjects/relay-agent-switch/crates/relay-core/src/adapters/codex/login.rs)
- [`SettingsView.swift`](/Users/hawa130/SoftwareProjects/relay-agent-switch/apps/relay-macos/RelayApp/Settings/SettingsView.swift)

## Design Rationale

The current design favors:

- a stable CLI protocol
- small, explicit data models
- separation between orchestration state and account identity
- clear source/confidence semantics for usage
- minimum coupling between UI and Codex-specific side effects

This keeps the project extensible without over-building a multi-agent platform before the Codex loop is solid.

## Known Tradeoffs

- some runtime truth is still split across SQLite and file caches
- `agent` persistence is structurally present but still effectively Codex-only in practice
- remote usage probing currently depends on provider-specific identity material stored per profile

## Recommended Next Steps

1. Consolidate more runtime state into a clearer transactional boundary.
2. Introduce a more formal provider abstraction for future non-Codex agents.
3. Add explicit documentation for the `profiles login codex` and `profiles relink codex` lifecycle.
4. Expand technical docs for agent-scoped usage source selection rules and Codex settings flows.
