use crate::models::{RelayError, UsageSnapshot};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileUsageStore {
    usage_path: PathBuf,
}

impl FileUsageStore {
    pub fn new(usage_path: impl AsRef<Path>) -> Self {
        Self {
            usage_path: usage_path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<Option<UsageSnapshot>, RelayError> {
        if !self.usage_path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&self.usage_path)?;
        let snapshot = serde_json::from_str(&contents)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        Ok(Some(snapshot))
    }

    pub fn save(&self, snapshot: &UsageSnapshot) -> Result<(), RelayError> {
        if let Some(parent) = self.usage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self.usage_path.with_extension("tmp");
        let contents = serde_json::to_string_pretty(snapshot)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        fs::write(&temp_path, contents)?;
        fs::rename(temp_path, &self.usage_path)?;
        Ok(())
    }
}
