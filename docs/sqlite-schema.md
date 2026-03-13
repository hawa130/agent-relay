# SQLite Schema

AgentRelay uses a SeaORM 2.x entity-first workflow for managed SQLite schema. The hand-written entities inside `relay-core` are the schema source of truth, and write bootstrap uses SeaORM schema sync to create or extend the database.

## Current Policy

- `relay-core::store::entities` is the only source of truth for managed SQLite schema
- `SqliteStore::new` opens the database, rejects incompatible legacy schemas, and runs SeaORM schema sync before service logic proceeds in write mode
- read-only bootstrap can open an existing synchronized database without running schema sync
- unsupported legacy database layouts must be recreated instead of patched in place

## Current Baseline

The current schema includes:

- `profiles`
- `app_settings`
- `switch_history`
- `failure_events`
- `profile_probe_identities`
- `agent_settings`

Schema changes should be made by editing the hand-written entities in `relay-core`.

## Schema Rules

When introducing a schema change:

1. update the relevant entities under `relay-core::store::entities`
2. keep bootstrap failure behavior stable for callers
3. update store and service logic that depends on the changed schema
4. add or update tests for:
   - fresh bootstrap on an empty database
   - refusal of legacy or unsupported database states
   - read-only bootstrap against an already synchronized database

## Operational Notes

- during development, delete `relay.db` when a breaking schema change requires a clean local bootstrap
- if schema sync or schema validation fails, startup must fail before any user-visible mutation proceeds
