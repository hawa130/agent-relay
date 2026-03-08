use crate::adapters::CodexAdapter;
use crate::models::{ActiveState, AppSettings, RelayError, StatusReport};
use crate::platform::RelayPaths;
use crate::store::SqliteStore;

pub fn build(
    paths: &RelayPaths,
    store: &SqliteStore,
    active_state: ActiveState,
    settings: AppSettings,
    adapter: &CodexAdapter,
) -> Result<StatusReport, RelayError> {
    let profiles = store.list_profiles()?;

    Ok(StatusReport {
        relay_home: paths.relay_home.to_string_lossy().into_owned(),
        live_agent_home: adapter.live_home().to_string_lossy().into_owned(),
        profile_count: profiles.len(),
        active_state,
        settings,
    })
}
