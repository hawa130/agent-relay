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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{UsageConfidence, UsageSource, UsageStatus, UsageWindow};
    use chrono::Utc;
    use tempfile::tempdir;

    fn test_snapshot(profile_id: Option<&str>) -> UsageSnapshot {
        UsageSnapshot {
            profile_id: profile_id.map(str::to_string),
            profile_name: profile_id.map(|id| format!("name-{id}")),
            source: UsageSource::Local,
            confidence: UsageConfidence::High,
            stale: false,
            last_refreshed_at: Utc::now(),
            next_reset_at: None,
            session: UsageWindow {
                used_percent: Some(10.0),
                window_minutes: Some(300),
                reset_at: None,
                status: UsageStatus::Healthy,
                exact: true,
            },
            weekly: UsageWindow {
                used_percent: Some(5.0),
                window_minutes: Some(10080),
                reset_at: None,
                status: UsageStatus::Healthy,
                exact: true,
            },
            auto_switch_reason: None,
            can_auto_switch: false,
            message: None,
            remote_error: None,
            plan_hint: None,
        }
    }

    #[tokio::test]
    async fn save_and_load_profile_round_trip() {
        let temp = tempdir().expect("tempdir");
        let store = FileUsageStore::new(temp.path().join("usage.json"));
        let snapshot = test_snapshot(Some("p1"));

        store.save_profile(&snapshot).await.expect("save");
        let loaded = store.load_profile("p1").await.expect("load");

        let loaded = loaded.expect("should find profile");
        assert_eq!(loaded.profile_id.as_deref(), Some("p1"));
        assert_eq!(loaded.session.used_percent, Some(10.0));
    }

    #[tokio::test]
    async fn save_all_and_load_all_round_trip() {
        let temp = tempdir().expect("tempdir");
        let store = FileUsageStore::new(temp.path().join("usage.json"));
        let snapshots = vec![test_snapshot(Some("a")), test_snapshot(Some("b"))];

        store.save_all(&snapshots).await.expect("save_all");
        let loaded = store.load_all().await.expect("load_all");

        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn load_all_on_missing_file_returns_empty() {
        let temp = tempdir().expect("tempdir");
        let store = FileUsageStore::new(temp.path().join("nonexistent.json"));

        let loaded = store.load_all().await.expect("load_all");

        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn save_profile_replaces_existing_for_same_id() {
        let temp = tempdir().expect("tempdir");
        let store = FileUsageStore::new(temp.path().join("usage.json"));

        let mut first = test_snapshot(Some("p1"));
        first.session.used_percent = Some(10.0);
        store.save_profile(&first).await.expect("save first");

        let mut second = test_snapshot(Some("p1"));
        second.session.used_percent = Some(90.0);
        store.save_profile(&second).await.expect("save second");

        let all = store.load_all().await.expect("load_all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].session.used_percent, Some(90.0));
    }

    #[tokio::test]
    async fn load_profile_returns_none_for_unknown() {
        let temp = tempdir().expect("tempdir");
        let store = FileUsageStore::new(temp.path().join("usage.json"));
        store
            .save_profile(&test_snapshot(Some("p1")))
            .await
            .expect("save");

        let loaded = store.load_profile("unknown").await.expect("load");

        assert!(loaded.is_none());
    }
}
