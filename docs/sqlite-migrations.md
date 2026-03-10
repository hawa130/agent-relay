# SQLite Migrations

Relay manages SQLite schema changes through embedded SeaORM migrations in `relay-core`.

## Current Policy

- `relay-core` owns the database schema, entities, and migration execution.
- `SqliteStore::new` opens the database and runs pending SeaORM migrations before any service logic proceeds.
- Relay no longer treats `PRAGMA user_version` as the source of truth for schema state. Migration state is stored in SeaORM's migration metadata table.
- A read-only bootstrap path may open an existing database without running migrations.

## Current Baseline

- The baseline migration creates:
  - `profiles`
  - `app_settings`
  - `switch_history`
  - `failure_events`
  - `profile_probe_identities`
  - `agent_settings`
- Future schema changes should be added as new embedded SeaORM migrations under `crates/relay-core/src/store/migrations`.

## Migration Rules

When introducing a new schema revision:

1. Add a new SeaORM migration module in `crates/relay-core/src/store/migrations`.
2. Keep the migration embedded in `relay-core`; do not add a separate migration crate.
3. Update or add the matching SeaORM entity definitions under `crates/relay-core/src/store/entities`.
4. Run migrations transactionally during bootstrap and keep caller-visible failure behavior stable.
5. Add or update tests for:
   - fresh bootstrap on an empty database
   - refusal or safe handling of unsupported database states

## Operational Notes

- Relay assumes the current embedded migration baseline during development.
- If migration bootstrap fails, startup must fail before any user-visible mutation proceeds.
