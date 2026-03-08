use crate::RelayError;
use crate::models::FailureEvent;
use crate::store::SqliteStore;

pub fn list_failure_events(
    store: &SqliteStore,
    limit: usize,
) -> Result<Vec<FailureEvent>, RelayError> {
    store.list_failure_events(limit)
}
