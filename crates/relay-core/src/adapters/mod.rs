pub mod codex;

use crate::app::AgentLoginMode;
use crate::models::{
    AgentKind, AgentLinkResult, Profile, ProfileProbeIdentity, RelayError, SwitchCheckpoint,
    UsageSnapshot,
};
use crate::platform::RelayPaths;
use crate::store::SqliteStore;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use codex::CodexAdapter;

#[async_trait(?Send)]
pub trait AgentAdapter {
    fn kind(&self) -> AgentKind;
    fn binary_name(&self) -> &'static str;
    fn home_env_var_name(&self) -> Option<&'static str>;
    fn default_home(&self) -> Option<PathBuf>;
    fn live_home(&self) -> &Path;
    fn managed_files(&self) -> Vec<String>;
    fn validate_profile(&self, profile: &Profile) -> Result<(), RelayError>;
    fn import_live_profile(&self, destination: &Path) -> Result<Vec<String>, RelayError>;
    async fn import_profile(
        &self,
        store: &SqliteStore,
        paths: &RelayPaths,
        nickname: Option<String>,
        priority: i32,
    ) -> Result<Profile, RelayError>;
    async fn login_profile(
        &self,
        store: &SqliteStore,
        profiles_dir: &Path,
        nickname: Option<String>,
        priority: i32,
        mode: AgentLoginMode,
    ) -> Result<AgentLinkResult, RelayError>;
    async fn relink_profile(
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

#[async_trait(?Send)]
pub trait UsageProvider {
    fn collect_local_usage(
        &self,
        target_profile: Option<&Profile>,
        active_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError>;
    async fn collect_remote_usage(
        &self,
        store: &SqliteStore,
        target_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError>;
}

trait RegisteredAgent: AgentAdapter + UsageProvider {}

impl<T> RegisteredAgent for T where T: AgentAdapter + UsageProvider {}

pub struct AdapterRegistry {
    primary_kind: AgentKind,
    entries: HashMap<AgentKind, Box<dyn RegisteredAgent>>,
}

impl AdapterRegistry {
    pub fn new() -> Result<Self, RelayError> {
        let mut entries: HashMap<AgentKind, Box<dyn RegisteredAgent>> = HashMap::new();
        entries.insert(AgentKind::Codex, Box::new(CodexAdapter::new()?));
        Ok(Self {
            primary_kind: AgentKind::Codex,
            entries,
        })
    }

    pub fn primary_kind(&self) -> AgentKind {
        self.primary_kind.clone()
    }

    pub fn primary(&self) -> &dyn AgentAdapter {
        self.adapter(&self.primary_kind)
    }

    pub fn primary_usage_provider(&self) -> &dyn UsageProvider {
        self.usage_provider(&self.primary_kind)
    }

    pub fn adapter(&self, kind: &AgentKind) -> &dyn AgentAdapter {
        self.registered(kind)
    }

    pub fn usage_provider(&self, kind: &AgentKind) -> &dyn UsageProvider {
        self.registered(kind)
    }

    fn registered(&self, kind: &AgentKind) -> &dyn RegisteredAgent {
        self.entries
            .get(kind)
            .map(Box::as_ref)
            .unwrap_or_else(|| panic!("adapter not registered for {:?}", kind))
    }
}
