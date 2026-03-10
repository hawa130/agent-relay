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
        let event = insert_failure_event(
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
            insert_failure_event(&transaction, profile_id, reason, message, cooldown_until).await?;
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

async fn insert_failure_event<C>(
    connection: &C,
    profile_id: Option<&str>,
    reason: FailureReason,
    message: impl AsRef<str>,
    cooldown_until: Option<DateTime<Utc>>,
) -> Result<FailureEvent, RelayError>
where
    C: ConnectionTrait,
{
    let event = FailureEvent {
        id: format!("ev_{}", Utc::now().timestamp_millis()),
        profile_id: profile_id.map(ToOwned::to_owned),
        reason: reason.clone(),
        message: message.as_ref().to_string(),
        cooldown_until,
        created_at: Utc::now(),
    };

    failure_events::ActiveModel {
        id: Set(event.id.clone()),
        profile_id: Set(event.profile_id.clone()),
        reason: Set(stringify_reason(&event.reason).to_string()),
        message: Set(event.message.clone()),
        cooldown_until: Set(event.cooldown_until.map(|value| value.to_rfc3339())),
        created_at: Set(event.created_at.to_rfc3339()),
    }
    .insert(connection)
    .await?;

    Ok(event)
}
