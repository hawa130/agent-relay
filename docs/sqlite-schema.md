# SQLite Schema

AgentRelay now manages SQLite schema changes with SeaORM 2.x entity-first workflow.
The hand-written entities inside `relay-core` are the schema source of truth,
and write bootstrap uses SeaORM schema sync to create or extend the database.

## Current Policy

- `relay-core::store::entities` is the only source of truth for managed SQLite schema.
- `SqliteStore::new` opens the database, rejects legacy schemas, and runs SeaORM schema sync before any service logic proceeds.
- A read-only bootstrap path may open an existing database without running schema sync.
- Existing migration-based AgentRelay databases are treated as incompatible and must be recreated.

## Current Baseline

- The current entity-first schema creates:
  - `profiles`
  - `app_settings`
  - `switch_history`
  - `failure_events`
  - `profile_probe_identities`
  - `agent_settings`
- Future schema changes should be made by editing the hand-written entities in `relay-core`.

## Schema Rules

When introducing a schema revision:

1. Update the relevant hand-written entities under `relay-core::store::entities`.
2. Keep bootstrap failure behavior stable for callers.
3. Add or update tests for:
   - fresh bootstrap on an empty database
   - refusal of legacy or unsupported database states
   - read-only bootstrap against an already synchronized database

## Operational Notes

- AgentRelay assumes the current entity-first schema baseline during development.
- If schema sync or schema validation fails, startup must fail before any user-visible mutation proceeds.
