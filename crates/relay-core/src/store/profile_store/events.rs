use super::*;

impl SqliteStore {
    pub async fn record_switch(
        &self,
        record: SwitchHistoryRecord,
    ) -> Result<SwitchHistoryEntry, RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let entry = insert_switch_history(&transaction, record).await?;
        transaction.commit().await?;
        Ok(entry)
    }

    pub async fn record_switch_failure(
        &self,
        record: SwitchHistoryRecord,
        failure_reason: FailureReason,
        failure_message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<(SwitchHistoryEntry, FailureEvent), RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let entry = insert_switch_history(&transaction, record).await?;
        let event = upsert_failure_event(
            &transaction,
            entry.profile_id.as_deref(),
            failure_reason,
            failure_message,
            cooldown_until,
        )
        .await?;
        transaction.commit().await?;
        Ok((entry, event))
    }

    pub async fn list_switch_history(
        &self,
        limit: usize,
    ) -> Result<Vec<SwitchHistoryEntry>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        switch_history::Entity::find()
            .order_by_desc(switch_history::Column::CreatedAt)
            .limit(limit as u64)
            .all(connection)
            .await?
            .into_iter()
            .map(switch_history_from_model)
            .collect()
    }

    pub async fn record_failure_event(
        &self,
        profile_id: Option<&str>,
        reason: FailureReason,
        message: impl AsRef<str>,
        cooldown_until: Option<DateTime<Utc>>,
    ) -> Result<FailureEvent, RelayError> {
        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let event =
            upsert_failure_event(&transaction, profile_id, reason, message, cooldown_until).await?;
        transaction.commit().await?;
        Ok(event)
    }

    pub async fn list_failure_events(&self, limit: usize) -> Result<Vec<FailureEvent>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        failure_events::Entity::find()
            .order_by_desc(failure_events::Column::CreatedAt)
            .limit(limit as u64)
            .all(connection)
            .await?
            .into_iter()
            .map(failure_event_from_model)
            .collect()
    }

    pub async fn list_current_failure_events(
        &self,
        profile_id: Option<&str>,
    ) -> Result<Vec<FailureEvent>, RelayError> {
        let Some(connection) = self.connection() else {
            return Ok(Vec::new());
        };

        let mut query = failure_events::Entity::find()
            .filter(failure_events::Column::ResolvedAt.is_null())
            .order_by_desc(failure_events::Column::CreatedAt);
        if let Some(profile_id) = profile_id {
            query = query.filter(failure_events::Column::ProfileId.eq(profile_id));
        }

        query
            .all(connection)
            .await?
            .into_iter()
            .map(failure_event_from_model)
            .collect()
    }

    pub async fn resolve_failure_events(
        &self,
        profile_id: &str,
        reasons: &[FailureReason],
    ) -> Result<Vec<FailureEvent>, RelayError> {
        if reasons.is_empty() {
            return Ok(Vec::new());
        }

        let connection = self.require_connection()?;
        let transaction = connection.begin().await?;
        let now = Utc::now();
        let mut resolved = Vec::new();

        for reason in reasons {
            let Some(model) = failure_events::Entity::find()
                .filter(failure_events::Column::ProfileId.eq(profile_id))
                .filter(failure_events::Column::Reason.eq(stringify_reason(reason)))
                .filter(failure_events::Column::ResolvedAt.is_null())
                .order_by_asc(failure_events::Column::CreatedAt)
                .one(&transaction)
                .await?
            else {
                continue;
            };

            let mut active = model.into_active_model();
            active.resolved_at = Set(Some(now.to_rfc3339()));
            let updated = active.update(&transaction).await?;
            resolved.push(failure_event_from_model(updated)?);
        }

        transaction.commit().await?;
        Ok(resolved)
    }

    #[cfg(test)]
    pub async fn record_failure_event_for_test(
        &self,
        profile_id: &str,
        reason: FailureReason,
        message: impl AsRef<str>,
    ) -> Result<FailureEvent, RelayError> {
        self.record_failure_event(Some(profile_id), reason, message, None)
            .await
    }
}

async fn insert_switch_history<C>(
    connection: &C,
    record: SwitchHistoryRecord,
) -> Result<SwitchHistoryEntry, RelayError>
where
    C: ConnectionTrait,
{
    let id = format!("sw_{}", Utc::now().timestamp_millis());
    let created_at = Utc::now();

    switch_history::ActiveModel {
        id: Set(id.clone()),
        profile_id: Set(record.profile_id.clone()),
        previous_profile_id: Set(record.previous_profile_id.clone()),
        outcome: Set(stringify_outcome(&record.outcome).to_string()),
        reason: Set(record.reason.clone()),
        checkpoint_id: Set(record.checkpoint_id.clone()),
        rollback_performed: Set(record.rollback_performed),
        created_at: Set(created_at.to_rfc3339()),
        details: Set(json!({}).to_string()),
    }
    .insert(connection)
    .await?;

    Ok(SwitchHistoryEntry {
        id,
        profile_id: record.profile_id,
        previous_profile_id: record.previous_profile_id,
        outcome: record.outcome,
        reason: record.reason,
        checkpoint_id: record.checkpoint_id,
        rollback_performed: record.rollback_performed,
        created_at,
    })
}

async fn upsert_failure_event<C>(
    connection: &C,
    profile_id: Option<&str>,
    reason: FailureReason,
    message: impl AsRef<str>,
    cooldown_until: Option<DateTime<Utc>>,
) -> Result<FailureEvent, RelayError>
where
    C: ConnectionTrait,
{
    if let Some(profile_id) = profile_id {
        if let Some(model) = failure_events::Entity::find()
            .filter(failure_events::Column::ProfileId.eq(profile_id))
            .filter(failure_events::Column::Reason.eq(stringify_reason(&reason)))
            .filter(failure_events::Column::ResolvedAt.is_null())
            .order_by_asc(failure_events::Column::CreatedAt)
            .one(connection)
            .await?
        {
            let mut active = model.into_active_model();
            active.message = Set(message.as_ref().to_string());
            active.cooldown_until = Set(cooldown_until.map(|value| value.to_rfc3339()));
            let updated = active.update(connection).await?;
            return failure_event_from_model(updated);
        }
    }

    let event = FailureEvent {
        id: format!("ev_{}", Utc::now().timestamp_millis()),
        profile_id: profile_id.map(ToOwned::to_owned),
        reason: reason.clone(),
        message: message.as_ref().to_string(),
        cooldown_until,
        resolved_at: None,
        created_at: Utc::now(),
    };

    failure_events::ActiveModel {
        id: Set(event.id.clone()),
        profile_id: Set(event.profile_id.clone()),
        reason: Set(stringify_reason(&event.reason).to_string()),
        message: Set(event.message.clone()),
        cooldown_until: Set(event.cooldown_until.map(|value| value.to_rfc3339())),
        resolved_at: Set(None),
        created_at: Set(event.created_at.to_rfc3339()),
    }
    .insert(connection)
    .await?;

    Ok(event)
}
