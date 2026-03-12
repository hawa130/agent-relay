# Security Review Checklist

Use this checklist before releases that change switching, storage, diagnostics, or agent adapter behavior.

## Secrets

- Confirm AgentRelay does not log raw auth tokens, cookies, or credential blobs.
- Confirm diagnostics export redacts secrets and sensitive user paths where intended.
- Confirm SQLite records do not persist plaintext credentials beyond documented metadata.
- Confirm example commands and docs avoid encouraging unsafe secret handling.

## Switching And Rollback

- Confirm switch writes use temp files plus atomic replace semantics.
- Confirm a failed post-switch validation restores the previous live config.
- Confirm checkpoint metadata is sufficient to explain what was backed up and restored.
- Confirm rollback failures are surfaced as stable error codes and logged clearly.
- Confirm project-local `.codex/` remains untouched.

## Filesystem Boundaries

- Confirm AgentRelay writes only under `AGRELAY_HOME` and the user-level live Codex home.
- Confirm profile removal only deletes AgentRelay-managed profile directories.
- Confirm diagnostics export stays within AgentRelay-managed export paths.

## CLI / JSON Contract

- Confirm user-visible command failures map to stable `ErrorCode` values.
- Confirm every external command path used by adapters is explicit and validated.
- Confirm `--json` output remains parseable and backward-compatible for existing fields.

## Release Gate

- Run `cargo fmt --all --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test`
- Smoke test `agrelay doctor --json`
- Smoke test one successful switch and one rollback path with temp homes
