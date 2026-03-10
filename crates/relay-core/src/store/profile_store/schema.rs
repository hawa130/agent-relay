use crate::models::RelayError;
use crate::store::entities::{
    agent_settings, app_settings, failure_events, profile_probe_identities, profiles,
    switch_history,
};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Order, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, QuerySelect};
use std::path::Path;

const LEGACY_MIGRATIONS_TABLE: &str = "seaql_migrations";
pub(super) const MANAGED_SCHEMA: &[(&str, &[&str])] = &[
    (
        "profiles",
        &[
            "id",
            "nickname",
            "agent",
            "priority",
            "enabled",
            "agent_home",
            "config_path",
            "auth_mode",
            "metadata",
            "created_at",
            "updated_at",
        ],
    ),
    ("app_settings", &["key", "value"]),
    (
        "switch_history",
        &[
            "id",
            "profile_id",
            "previous_profile_id",
            "outcome",
            "reason",
            "checkpoint_id",
            "rollback_performed",
            "created_at",
            "details",
        ],
    ),
    (
        "failure_events",
        &[
            "id",
            "profile_id",
            "reason",
            "message",
            "cooldown_until",
            "created_at",
        ],
    ),
    (
        "profile_probe_identities",
        &[
            "profile_id",
            "provider",
            "principal_id",
            "display_name",
            "credentials_json",
            "metadata_json",
            "created_at",
            "updated_at",
        ],
    ),
    (
        "agent_settings",
        &["agent", "settings_json", "created_at", "updated_at"],
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SchemaState {
    Empty,
    Syncable,
    Ready,
    Legacy,
    Incompatible,
}

#[cfg(test)]
pub(super) async fn has_table(
    connection: &DatabaseConnection,
    table: &str,
) -> Result<bool, RelayError> {
    let query = Query::select()
        .expr(Expr::val(1))
        .from(Alias::new("sqlite_master"))
        .and_where(Expr::col(Alias::new("type")).eq("table"))
        .and_where(Expr::col(Alias::new("name")).eq(table))
        .limit(1)
        .to_owned();
    Ok(connection.query_one(&query).await?.is_some())
}

pub(super) async fn inspect_schema_state(
    connection: &DatabaseConnection,
) -> Result<SchemaState, RelayError> {
    let user_tables = list_user_tables(connection).await?;
    if user_tables.is_empty() {
        return Ok(SchemaState::Empty);
    }

    if user_tables
        .iter()
        .any(|table| table == LEGACY_MIGRATIONS_TABLE)
    {
        return Ok(SchemaState::Legacy);
    }

    let mut missing_schema = false;
    for (table, _) in MANAGED_SCHEMA {
        if !user_tables.iter().any(|present| present == table) {
            missing_schema = true;
        }
    }

    if user_tables
        .iter()
        .any(|table| !MANAGED_SCHEMA.iter().any(|(managed, _)| managed == table))
    {
        Ok(SchemaState::Incompatible)
    } else if missing_schema {
        Ok(SchemaState::Syncable)
    } else {
        Ok(SchemaState::Ready)
    }
}

async fn list_user_tables(connection: &DatabaseConnection) -> Result<Vec<String>, RelayError> {
    let query = Query::select()
        .column(Alias::new("name"))
        .from(Alias::new("sqlite_master"))
        .and_where(Expr::col(Alias::new("type")).eq("table"))
        .and_where(Expr::col(Alias::new("name")).not_like("sqlite_%"))
        .order_by(Alias::new("name"), Order::Asc)
        .to_owned();
    Ok(connection
        .query_all(&query)
        .await?
        .into_iter()
        .filter_map(|row| row.try_get::<String>("", "name").ok())
        .collect())
}

pub(super) async fn sync_schema(connection: &DatabaseConnection) -> Result<(), RelayError> {
    connection
        .get_schema_builder()
        .register(profiles::Entity)
        .register(app_settings::Entity)
        .register(switch_history::Entity)
        .register(failure_events::Entity)
        .register(profile_probe_identities::Entity)
        .register(agent_settings::Entity)
        .sync(connection)
        .await?;
    Ok(())
}

pub(super) async fn validate_schema_queries(
    connection: &DatabaseConnection,
) -> Result<(), RelayError> {
    profiles::Entity::find().limit(1).all(connection).await?;
    app_settings::Entity::find()
        .limit(1)
        .all(connection)
        .await?;
    switch_history::Entity::find()
        .limit(1)
        .all(connection)
        .await?;
    failure_events::Entity::find()
        .limit(1)
        .all(connection)
        .await?;
    profile_probe_identities::Entity::find()
        .limit(1)
        .all(connection)
        .await?;
    agent_settings::Entity::find()
        .limit(1)
        .all(connection)
        .await?;
    Ok(())
}

pub(super) fn schema_incompatible_error() -> RelayError {
    RelayError::SchemaIncompatible(
        "relay database schema is incompatible with this build; remove the existing database and let relay recreate it"
            .into(),
    )
}

pub(super) fn sqlite_url(path: &Path, read_only: bool) -> String {
    let mode = if read_only { "?mode=ro" } else { "?mode=rwc" };
    format!("sqlite://{}{}", path.to_string_lossy(), mode)
}
