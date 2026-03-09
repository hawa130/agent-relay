pub mod codex;

use crate::models::{
    AgentKind, AgentLinkResult, Profile, ProfileProbeIdentity, RelayError, SwitchCheckpoint,
    UsageSnapshot,
};
use crate::platform::RelayPaths;
use crate::store::SqliteStore;
use std::path::{Path, PathBuf};

pub use codex::CodexAdapter;

pub trait AgentAdapter {
    fn kind(&self) -> AgentKind;
    fn binary_name(&self) -> &'static str;
    fn default_home(&self) -> Option<PathBuf>;
    fn live_home(&self) -> &Path;
    fn managed_files(&self) -> Vec<String>;
    fn validate_profile(&self, profile: &Profile) -> Result<(), RelayError>;
    fn import_live_profile(&self, destination: &Path) -> Result<Vec<String>, RelayError>;
    fn import_profile(
        &self,
        store: &SqliteStore,
        paths: &RelayPaths,
        nickname: Option<String>,
        priority: i32,
    ) -> Result<Profile, RelayError>;
    fn login_profile(
        &self,
        store: &SqliteStore,
        profiles_dir: &Path,
        nickname: Option<String>,
        priority: i32,
    ) -> Result<AgentLinkResult, RelayError>;
    fn relink_profile(
        &self,
        store: &SqliteStore,
        profile: &Profile,
    ) -> Result<ProfileProbeIdentity, RelayError>;
    fn activate(
        &self,
        profile: &Profile,
        snapshot_root: &Path,
    ) -> Result<SwitchCheckpoint, RelayError>;
    fn rollback_checkpoint(
        &self,
        snapshot_root: &Path,
        checkpoint_id: &str,
    ) -> Result<(), RelayError>;
}

pub trait UsageProvider {
    fn collect_local_usage(
        &self,
        target_profile: Option<&Profile>,
        active_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError>;
    fn collect_remote_usage(
        &self,
        store: &SqliteStore,
        target_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError>;
}

#[derive(Debug, Clone)]
pub struct AdapterRegistry {
    codex: CodexAdapter,
}

impl AdapterRegistry {
    pub fn new() -> Result<Self, RelayError> {
        Ok(Self {
            codex: CodexAdapter::new()?,
        })
    }

    pub fn primary_kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    pub fn primary(&self) -> &dyn AgentAdapter {
        &self.codex
    }

    pub fn primary_usage_provider(&self) -> &dyn UsageProvider {
        &self.codex
    }

    pub fn adapter(&self, kind: &AgentKind) -> &dyn AgentAdapter {
        match kind {
            AgentKind::Codex => &self.codex,
        }
    }

    pub fn usage_provider(&self, kind: &AgentKind) -> &dyn UsageProvider {
        match kind {
            AgentKind::Codex => &self.codex,
        }
    }
}
