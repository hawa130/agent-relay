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

    pub fn load(&self) -> Result<ActiveState, RelayError> {
        if !self.state_path.exists() {
            return Ok(ActiveState::default());
        }

        let contents = fs::read_to_string(&self.state_path)?;
        let state = serde_json::from_str(&contents)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(state)
    }

    pub fn save(&self, state: &ActiveState) -> Result<(), RelayError> {
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

    #[test]
    fn save_and_load_state() {
        let temp = tempdir().expect("tempdir");
        let store = FileStateStore::new(temp.path().join("state.json"));
        let state = ActiveState {
            auto_switch_enabled: true,
            last_error: Some("boom".into()),
            ..ActiveState::default()
        };

        store.save(&state).expect("save");
        let loaded = store.load().expect("load");
        assert!(loaded.auto_switch_enabled);
        assert_eq!(loaded.last_error.as_deref(), Some("boom"));
    }
}
