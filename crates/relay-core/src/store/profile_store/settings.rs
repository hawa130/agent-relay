use super::*;

impl SqliteStore {
    pub(super) async fn ensure_default_settings(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<(), RelayError> {
        let defaults = AppSettings::default();
        self.ensure_setting_default(
            connection,
            "auto_switch_enabled",
            if defaults.auto_switch_enabled {
                "true"
            } else {
                "false"
            },
        )
        .await?;
        self.ensure_setting_default(
            connection,
            "cooldown_seconds",
            &defaults.cooldown_seconds.to_string(),
        )
        .await?;
        self.ensure_setting_default(
            connection,
            "refresh_interval_seconds",
            &defaults.refresh_interval_seconds.to_string(),
        )
        .await?;
        self.ensure_setting_default(
            connection,
            "network_query_concurrency",
            &defaults.network_query_concurrency.to_string(),
        )
        .await?;

        if let Some(model) = app_settings::Entity::find_by_id("usage_source_mode")
            .one(connection)
            .await?
        {
            model.delete(connection).await?;
        }
        Ok(())
    }

    pub(super) async fn ensure_default_agent_settings(
        &self,
        connection: &DatabaseConnection,
    ) -> Result<(), RelayError> {
        if agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
            .one(connection)
            .await?
            .is_some()
        {
            return Ok(());
        }

        let defaults = CodexSettings::default();
        let timestamp = Utc::now().to_rfc3339();
        agent_settings::ActiveModel {
            agent: Set(stringify_agent_kind(&AgentKind::Codex).to_string()),
            settings_json: Set(serde_json::to_string(&defaults)
                .map_err(|error| RelayError::Store(error.to_string()))?),
            created_at: Set(timestamp.clone()),
            updated_at: Set(timestamp),
        }
        .insert(connection)
        .await?;
        Ok(())
    }

    pub async fn get_settings(&self) -> Result<AppSettings, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(AppSettings::default());
        };
        let auto_switch_enabled = app_settings::Entity::find_by_id("auto_switch_enabled")
            .one(connection)
            .await?
            .map(|value| value.value == "true")
            .unwrap_or(false);
        let cooldown_seconds = app_settings::Entity::find_by_id("cooldown_seconds")
            .one(connection)
            .await?
            .and_then(|value| value.value.parse::<i64>().ok())
            .unwrap_or(600);
        let refresh_interval_seconds = app_settings::Entity::find_by_id("refresh_interval_seconds")
            .one(connection)
            .await?
            .and_then(|value| value.value.parse::<i64>().ok())
            .unwrap_or(60);
        let network_query_concurrency =
            app_settings::Entity::find_by_id("network_query_concurrency")
                .one(connection)
                .await?
                .and_then(|value| value.value.parse::<i64>().ok())
                .unwrap_or(10);

        Ok(AppSettings {
            auto_switch_enabled,
            cooldown_seconds,
            refresh_interval_seconds,
            network_query_concurrency,
        })
    }

    pub async fn set_auto_switch_enabled(&self, enabled: bool) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(
            connection,
            "auto_switch_enabled",
            if enabled { "true" } else { "false" },
        )
        .await?;
        self.get_settings().await
    }

    pub async fn set_cooldown_seconds(&self, value: i64) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(connection, "cooldown_seconds", &value.to_string())
            .await?;
        self.get_settings().await
    }

    pub async fn set_refresh_interval_seconds(
        &self,
        value: i64,
    ) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(connection, "refresh_interval_seconds", &value.to_string())
            .await?;
        self.get_settings().await
    }

    pub async fn set_network_query_concurrency(
        &self,
        value: i64,
    ) -> Result<AppSettings, RelayError> {
        let connection = self.require_connection()?;
        self.set_setting_value(connection, "network_query_concurrency", &value.to_string())
            .await?;
        self.get_settings().await
    }

    pub async fn codex_settings(&self) -> Result<CodexSettings, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(CodexSettings::default());
        };

        agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
            .one(connection)
            .await?
            .map(|value| {
                serde_json::from_str::<CodexSettings>(&value.settings_json)
                    .map_err(|error| RelayError::Store(error.to_string()))
            })
            .transpose()?
            .map(Ok)
            .unwrap_or_else(|| Ok(CodexSettings::default()))
    }

    pub async fn update_codex_settings(
        &self,
        request: CodexSettingsUpdateRequest,
    ) -> Result<CodexSettings, RelayError> {
        let current = self.codex_settings().await?;
        let settings = CodexSettings {
            usage_source_mode: request
                .usage_source_mode
                .unwrap_or(current.usage_source_mode),
        };
        let payload = serde_json::to_string(&settings)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let timestamp = Utc::now().to_rfc3339();
        let connection = self.require_connection()?;

        if let Some(model) =
            agent_settings::Entity::find_by_id(stringify_agent_kind(&AgentKind::Codex))
                .one(connection)
                .await?
        {
            let mut active = model.into_active_model();
            active.settings_json = Set(payload);
            active.updated_at = Set(timestamp);
            active.update(connection).await?;
        } else {
            agent_settings::ActiveModel {
                agent: Set(stringify_agent_kind(&AgentKind::Codex).to_string()),
                settings_json: Set(payload),
                created_at: Set(timestamp.clone()),
                updated_at: Set(timestamp),
            }
            .insert(connection)
            .await?;
        }

        self.codex_settings().await
    }

    async fn ensure_setting_default(
        &self,
        connection: &DatabaseConnection,
        key: &str,
        value: &str,
    ) -> Result<(), RelayError> {
        if app_settings::Entity::find_by_id(key.to_string())
            .one(connection)
            .await?
            .is_some()
        {
            return Ok(());
        }

        app_settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
        }
        .insert(connection)
        .await?;
        Ok(())
    }

    async fn set_setting_value(
        &self,
        connection: &DatabaseConnection,
        key: &str,
        value: &str,
    ) -> Result<(), RelayError> {
        if let Some(model) = app_settings::Entity::find_by_id(key.to_string())
            .one(connection)
            .await?
        {
            let mut active = model.into_active_model();
            active.value = Set(value.to_string());
            active.update(connection).await?;
        } else {
            app_settings::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_string()),
            }
            .insert(connection)
            .await?;
        }
        Ok(())
    }
}
