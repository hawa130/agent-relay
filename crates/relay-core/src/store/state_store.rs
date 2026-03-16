use crate::models::{ActiveState, RelayError};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileStateStore {
    state_path: PathBuf,
}

impl FileStateStore {
    pub fn new(state_path: impl AsRef<Path>) -> Self {
        Self {
            state_path: state_path.as_ref().to_path_buf(),
        }
    }

    pub async fn load(&self) -> Result<ActiveState, RelayError> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.load_sync())
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    pub async fn save(&self, state: &ActiveState) -> Result<(), RelayError> {
        let store = self.clone();
        let state = state.clone();
        tokio::task::spawn_blocking(move || store.save_sync(&state))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    fn load_sync(&self) -> Result<ActiveState, RelayError> {
        if !self.state_path.exists() {
            return Ok(ActiveState::default());
        }

        let contents = fs::read_to_string(&self.state_path)?;
        let state = serde_json::from_str(&contents)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(state)
    }

    fn save_sync(&self, state: &ActiveState) -> Result<(), RelayError> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self.state_path.with_extension("tmp");
        let contents = serde_json::to_string_pretty(state)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        fs::write(&temp_path, contents)?;
        fs::rename(temp_path, &self.state_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn save_and_load_state() {
        let temp = tempdir().expect("tempdir");
        let store = FileStateStore::new(temp.path().join("state.json"));
        let state = ActiveState {
            auto_switch_enabled: true,
            ..ActiveState::default()
        };

        store.save(&state).await.expect("save");
        let loaded = store.load().await.expect("load");
        assert!(loaded.auto_switch_enabled);
    }
}
