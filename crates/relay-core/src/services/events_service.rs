use crate::RelayError;
use crate::models::FailureEvent;
use crate::store::SqliteStore;

pub async fn list_failure_events(
    store: &SqliteStore,
    limit: usize,
) -> Result<Vec<FailureEvent>, RelayError> {
    store.list_failure_events(limit).await
}
