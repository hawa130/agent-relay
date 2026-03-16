use crate::models::{RelayError, UsageCache, UsageSnapshot};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct FileUsageStore {
    usage_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl FileUsageStore {
    pub fn new(usage_path: impl AsRef<Path>) -> Self {
        Self {
            usage_path: usage_path.as_ref().to_path_buf(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn load_all(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.load_all_sync())
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    pub async fn load_profile(
        &self,
        profile_id: &str,
    ) -> Result<Option<UsageSnapshot>, RelayError> {
        let store = self.clone();
        let profile_id = profile_id.to_string();
        tokio::task::spawn_blocking(move || store.load_profile_sync(&profile_id))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    pub async fn save_profile(&self, snapshot: &UsageSnapshot) -> Result<(), RelayError> {
        let store = self.clone();
        let snapshot = snapshot.clone();
        tokio::task::spawn_blocking(move || store.save_profile_sync(&snapshot))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    pub async fn save_all(&self, snapshots: &[UsageSnapshot]) -> Result<(), RelayError> {
        let store = self.clone();
        let snapshots = snapshots.to_vec();
        tokio::task::spawn_blocking(move || store.save_all_sync(&snapshots))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    fn load_all_sync(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        self.load_all_unlocked()
    }

    fn load_profile_sync(&self, profile_id: &str) -> Result<Option<UsageSnapshot>, RelayError> {
        Ok(self
            .load_all_sync()?
            .into_iter()
            .find(|snapshot| snapshot.profile_id.as_deref() == Some(profile_id)))
    }

    fn save_profile_sync(&self, snapshot: &UsageSnapshot) -> Result<(), RelayError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        let mut snapshots = self.load_all_unlocked()?;
        if let Some(profile_id) = snapshot.profile_id.as_deref() {
            snapshots.retain(|existing| existing.profile_id.as_deref() != Some(profile_id));
        } else {
            snapshots.retain(|existing| existing.profile_id.is_some());
        }
        snapshots.push(snapshot.clone());
        self.save_all_unlocked(&snapshots)
    }

    fn save_all_sync(&self, snapshots: &[UsageSnapshot]) -> Result<(), RelayError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|error| RelayError::Store(error.to_string()))?;
        self.save_all_unlocked(snapshots)
    }

    fn load_all_unlocked(&self) -> Result<Vec<UsageSnapshot>, RelayError> {
        if !self.usage_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.usage_path)?;
        if let Ok(cache) = serde_json::from_str::<UsageCache>(&contents) {
            return Ok(cache.snapshots);
        }

        if let Ok(snapshot) = serde_json::from_str::<UsageSnapshot>(&contents) {
            return Ok(vec![snapshot]);
        }

        Err(RelayError::Store("failed to decode usage cache".into()))
    }

    fn save_all_unlocked(&self, snapshots: &[UsageSnapshot]) -> Result<(), RelayError> {
        if let Some(parent) = self.usage_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self.usage_path.with_extension("tmp");
        let cache = UsageCache {
            snapshots: snapshots.to_vec(),
        };
        let contents = serde_json::to_string_pretty(&cache)
            .map_err(|error| RelayError::Store(error.to_string()))?;
        fs::write(&temp_path, contents)?;
        fs::rename(temp_path, &self.usage_path)?;
        Ok(())
    }
}
