use super::*;

impl SqliteStore {
    pub async fn list_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        profile_entities::Entity::find()
            .order_by_asc(profile_entities::Column::Priority)
            .order_by_asc(profile_entities::Column::Nickname)
            .all(connection)
            .await?
            .into_iter()
            .map(profile_from_model)
            .collect()
    }

    pub async fn list_enabled_profiles(&self) -> Result<Vec<Profile>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        profile_entities::Entity::find()
            .filter(profile_entities::Column::Enabled.eq(true))
            .order_by_asc(profile_entities::Column::Priority)
            .order_by_asc(profile_entities::Column::Nickname)
            .all(connection)
            .await?
            .into_iter()
            .map(profile_from_model)
            .collect()
    }

    pub async fn get_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let Some(connection) = self.connection() else {
            return Err(RelayError::NotFound(format!("profile not found: {id}")));
        };

        profile_entities::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))
            .and_then(profile_from_model)
    }

    pub async fn add_profile(&self, record: AddProfileRecord) -> Result<Profile, RelayError> {
        let connection = self.require_connection()?;
        let now = Utc::now().to_rfc3339();
        let id = format!(
            "p_{}_{}",
            Utc::now().timestamp_millis(),
            slugify(&record.nickname)
        );

        profile_entities::ActiveModel {
            id: Set(id.clone()),
            nickname: Set(record.nickname),
            agent: Set(stringify_agent_kind(&record.agent).to_string()),
            priority: Set(record.priority),
            enabled: Set(true),
            account_state: Set(Some(
                stringify_profile_account_state(&crate::models::ProfileAccountState::Healthy)
                    .to_string(),
            )),
            account_error_http_status: Set(None),
            account_state_updated_at: Set(None),
            agent_home: Set(record.agent_home.as_ref().map(|path| path_to_string(path))),
            config_path: Set(record.config_path.as_ref().map(|path| path_to_string(path))),
            auth_mode: Set(stringify_auth_mode(&record.auth_mode).to_string()),
            metadata: Set(json!({}).to_string()),
            created_at: Set(now.clone()),
            updated_at: Set(now),
        }
        .insert(connection)
        .await?;

        self.get_profile(&id).await
    }

    pub async fn update_profile(
        &self,
        id: &str,
        update: ProfileUpdateRecord,
    ) -> Result<Profile, RelayError> {
        let current = self.get_profile(id).await?;
        let connection = self.require_connection()?;
        let model = profile_entities::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        let mut active = model.into_active_model();

        let nickname = update.nickname.unwrap_or(current.nickname);
        let priority = update.priority.unwrap_or(current.priority);
        let config_path = update
            .config_path
            .unwrap_or_else(|| current.config_path.map(PathBuf::from));
        let agent_home = update
            .agent_home
            .unwrap_or_else(|| current.agent_home.map(PathBuf::from));
        let auth_mode = update.auth_mode.unwrap_or(current.auth_mode);

        active.nickname = Set(nickname);
        active.priority = Set(priority);
        active.agent_home = Set(agent_home.as_ref().map(|path| path_to_string(path)));
        active.config_path = Set(config_path.as_ref().map(|path| path_to_string(path)));
        active.auth_mode = Set(stringify_auth_mode(&auth_mode).to_string());
        active.updated_at = Set(Utc::now().to_rfc3339());
        active.update(connection).await?;

        self.get_profile(id).await
    }

    pub async fn remove_profile(&self, id: &str) -> Result<Profile, RelayError> {
        let profile = self.get_profile(id).await?;
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;

        if let Some(identity) = profile_probe_identities::Entity::find_by_id(id.to_string())
            .one(&transaction)
            .await?
        {
            identity.delete(&transaction).await?;
        }

        let model = profile_entities::Entity::find_by_id(id.to_string())
            .one(&transaction)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        model.delete(&transaction).await?;
        transaction.commit().await?;

        Ok(profile)
    }

    pub async fn get_probe_identity(
        &self,
        profile_id: &str,
    ) -> Result<Option<ProfileProbeIdentity>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(None);
        };

        profile_probe_identities::Entity::find_by_id(profile_id.to_string())
            .one(connection)
            .await?
            .map(probe_identity_from_model)
            .transpose()
    }

    pub async fn upsert_probe_identity(
        &self,
        identity: &ProfileProbeIdentity,
    ) -> Result<ProfileProbeIdentity, RelayError> {
        let connection = self.require_connection()?;
        if let Some(model) =
            profile_probe_identities::Entity::find_by_id(identity.profile_id.clone())
                .one(connection)
                .await?
        {
            let mut active = model.into_active_model();
            active.provider = Set(stringify_probe_provider(&identity.provider).to_string());
            active.principal_id = Set(identity.principal_id.clone());
            active.display_name = Set(identity.display_name.clone());
            active.credentials_json = Set(identity.credentials.to_string());
            active.metadata_json = Set(identity.metadata.to_string());
            active.updated_at = Set(identity.updated_at.to_rfc3339());
            active.update(connection).await?;
        } else {
            profile_probe_identities::ActiveModel {
                profile_id: Set(identity.profile_id.clone()),
                provider: Set(stringify_probe_provider(&identity.provider).to_string()),
                principal_id: Set(identity.principal_id.clone()),
                display_name: Set(identity.display_name.clone()),
                credentials_json: Set(identity.credentials.to_string()),
                metadata_json: Set(identity.metadata.to_string()),
                created_at: Set(identity.created_at.to_rfc3339()),
                updated_at: Set(identity.updated_at.to_rfc3339()),
            }
            .insert(connection)
            .await?;
        }

        self.get_probe_identity(&identity.profile_id)
            .await?
            .ok_or_else(|| RelayError::Store("failed to reload probe identity".into()))
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<Profile, RelayError> {
        let connection = self.require_connection()?;
        let model = profile_entities::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        let mut active = model.into_active_model();
        active.enabled = Set(enabled);
        active.updated_at = Set(Utc::now().to_rfc3339());
        active.update(connection).await?;
        self.get_profile(id).await
    }

    pub async fn set_account_state(
        &self,
        id: &str,
        state: crate::models::ProfileAccountState,
        http_status: Option<u16>,
    ) -> Result<Profile, RelayError> {
        let connection = self.require_connection()?;
        let model = profile_entities::Entity::find_by_id(id.to_string())
            .one(connection)
            .await?
            .ok_or_else(|| RelayError::NotFound(format!("profile not found: {id}")))?;
        let mut active = model.into_active_model();
        active.account_state = Set(Some(stringify_profile_account_state(&state).to_string()));
        active.account_error_http_status = Set(http_status.map(i32::from));
        active.account_state_updated_at = Set(Some(Utc::now().to_rfc3339()));
        active.updated_at = Set(Utc::now().to_rfc3339());
        active.update(connection).await?;
        self.get_profile(id).await
    }

    pub async fn clear_account_state(&self, id: &str) -> Result<Profile, RelayError> {
        self.set_account_state(id, crate::models::ProfileAccountState::Healthy, None)
            .await
    }
}
