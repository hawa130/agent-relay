# SQLite Schema Versioning

Relay uses SQLite `PRAGMA user_version` as the single schema version marker for `relay.db`.

## Current Policy

- Schema changes are additive and versioned in a single ordered migration chain.
- `relay-core` owns migrations; callers do not run ad hoc SQL.
- `SqliteStore::new` performs automatic bootstrap and migration before any service logic runs.
- A Relay binary must refuse to open a database whose `user_version` is newer than the binary supports.
- The schema version applies to the whole database, not per-table revisions.

## Current Version

- `user_version = 1`
- Version 1 creates:
  - `profiles`
  - `app_settings`
  - `switch_history`
  - `failure_events`

## Migration Rules

When introducing schema version `N + 1`:

1. Add a new migration step in `crates/relay-core/src/store/profile_store.rs`.
2. Keep prior migrations intact; do not rewrite old steps after release.
3. Prefer additive changes and explicit backfills inside one transaction.
4. Update `CURRENT_SCHEMA_VERSION`.
5. Add or update tests for:
   - bootstrap on a fresh database
   - upgrade from the previous released schema
   - refusal to open a newer unknown schema
6. Document the change in release notes if the upgrade affects rollback, exports, or CLI compatibility.

## Operational Notes

- Existing pre-versioned local databases are treated as legacy version 0 and are stamped to version 1 during bootstrap.
- Relay does not currently provide a manual downgrade path.
- If a migration fails, startup must fail before any user-visible mutation proceeds.
