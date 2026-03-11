use super::schema::{MANAGED_SCHEMA, has_table, sqlite_url};
use super::*;
use crate::models::UsageSourceMode;
use sea_orm::Database;
use tempfile::tempdir;

#[tokio::test]
async fn profile_settings_events_and_switch_history_round_trip() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("relay.db");
    let store = SqliteStore::new(&db_path).await.expect("store");
    for (table, _) in MANAGED_SCHEMA {
        assert!(
            has_table(store.require_connection().expect("connection"), table)
                .await
                .expect("managed table"),
            "missing managed table: {table}"
        );
    }

    let created = store
        .add_profile(AddProfileRecord {
            agent: AgentKind::Codex,
            nickname: "Work".into(),
            priority: 10,
            config_path: Some(temp.path().join("config.toml")),
            agent_home: Some(temp.path().join(".codex-work")),
            auth_mode: crate::models::AuthMode::ConfigFilesystem,
        })
        .await
        .expect("profile");
    assert!(created.enabled);

    let updated = store
        .update_profile(
            &created.id,
            ProfileUpdateRecord {
                nickname: Some("Work Updated".into()),
                ..ProfileUpdateRecord::default()
            },
        )
        .await
        .expect("updated");
    assert_eq!(updated.nickname, "Work Updated");

    let settings = store.set_auto_switch_enabled(true).await.expect("settings");
    assert!(settings.auto_switch_enabled);

    let event = store
        .record_failure_event(
            Some(&created.id),
            FailureReason::ValidationFailed,
            "validation failed",
            None,
        )
        .await
        .expect("event");
    assert_eq!(event.profile_id.as_deref(), Some(created.id.as_str()));

    let switch_entry = store
        .record_switch(SwitchHistoryRecord {
            profile_id: Some(created.id.clone()),
            previous_profile_id: None,
            outcome: SwitchOutcome::Success,
            reason: Some("manual".into()),
            checkpoint_id: Some("ckpt".into()),
            rollback_performed: false,
        })
        .await
        .expect("switch");
    assert_eq!(switch_entry.checkpoint_id.as_deref(), Some("ckpt"));

    assert_eq!(
        store.list_failure_events(10).await.expect("events").len(),
        1
    );
    assert_eq!(
        store.list_switch_history(10).await.expect("history").len(),
        1
    );
    assert_eq!(store.list_profiles().await.expect("profiles").len(), 1);
}

#[tokio::test]
async fn codex_settings_round_trip() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("relay.db");
    let store = SqliteStore::new(&db_path).await.expect("store");

    let defaults = store
        .codex_settings()
        .await
        .expect("default codex settings");
    assert_eq!(defaults.usage_source_mode, UsageSourceMode::Auto);

    let updated = store
        .update_codex_settings(CodexSettingsUpdateRequest {
            usage_source_mode: Some(UsageSourceMode::WebEnhanced),
        })
        .await
        .expect("updated codex settings");
    assert_eq!(updated.usage_source_mode, UsageSourceMode::WebEnhanced);
    assert_eq!(
        store
            .codex_settings()
            .await
            .expect("reloaded codex settings"),
        updated
    );
}

#[tokio::test]
async fn app_settings_persist_across_store_reopen() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("relay.db");

    let store = SqliteStore::new(&db_path).await.expect("store");
    let updated = store
        .set_auto_switch_enabled(true)
        .await
        .expect("enable auto switch");
    assert!(updated.auto_switch_enabled);
    let updated = store
        .set_refresh_interval_seconds(120)
        .await
        .expect("set refresh interval");
    assert_eq!(updated.refresh_interval_seconds, 120);
    let updated = store
        .set_refresh_interval_seconds(0)
        .await
        .expect("disable refresh interval");
    assert_eq!(updated.refresh_interval_seconds, 0);
    drop(store);

    let reopened = SqliteStore::new(&db_path).await.expect("reopened store");
    let settings = reopened.get_settings().await.expect("reloaded settings");
    assert!(settings.auto_switch_enabled);
    assert_eq!(settings.refresh_interval_seconds, 0);
}

#[tokio::test]
async fn legacy_schema_is_rejected_in_read_write_and_read_only_modes() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("relay.db");
    let connection = Database::connect(sqlite_url(&db_path, false))
        .await
        .expect("legacy db");
    connection
        .execute_unprepared("CREATE TABLE seaql_migrations (version TEXT PRIMARY KEY NOT NULL)")
        .await
        .expect("create legacy table");

    let error = SqliteStore::new(&db_path)
        .await
        .expect_err("legacy schema error");
    assert!(matches!(error, RelayError::SchemaIncompatible(_)));

    let error = SqliteStore::open_read_only(&db_path)
        .await
        .expect_err("legacy read-only schema error");
    assert!(matches!(error, RelayError::SchemaIncompatible(_)));
}

#[tokio::test]
async fn read_only_bootstrap_treats_empty_existing_database_as_uninitialized() {
    let temp = tempdir().expect("tempdir");
    let db_path = temp.path().join("relay.db");
    let _connection = Database::connect(sqlite_url(&db_path, false))
        .await
        .expect("empty db");

    let store = SqliteStore::open_read_only(&db_path)
        .await
        .expect("read-only store");
    assert!(store.list_profiles().await.expect("profiles").is_empty());
}
