mod auth;
mod login;
mod recover;
mod settings;
mod usage;

use crate::adapters::{AgentAdapter, UsageProvider};
use crate::app::AgentLoginMode;
use crate::models::{
    AgentKind, AgentLinkResult, Profile, ProfileProbeIdentity, ProfileRecoveryReport, RelayError,
    SwitchCheckpoint, UsageSnapshot,
};
use crate::platform::{RelayPaths, find_binary};
use crate::store::SqliteStore;
use async_trait::async_trait;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

pub use settings::{CodexSettings, CodexSettingsUpdateRequest};

const REQUIRED_MANAGED_FILES: [&str; 1] = ["config.toml"];
const OPTIONAL_MANAGED_FILES: [&str; 2] = ["auth.json", "version.json"];

#[derive(Debug, Clone)]
pub struct CodexAdapter {
    live_home: PathBuf,
}

#[derive(Debug, Clone)]
struct ManagedSourceFiles {
    config: PathBuf,
    auth: Option<PathBuf>,
    version: Option<PathBuf>,
}

impl CodexAdapter {
    pub fn new() -> Result<Self, RelayError> {
        let live_home = live_codex_home()
            .ok_or_else(|| RelayError::Validation("failed to resolve live Codex home".into()))?;
        Ok(Self { live_home })
    }

    pub fn with_live_home(path: impl AsRef<Path>) -> Self {
        Self {
            live_home: path.as_ref().to_path_buf(),
        }
    }

    pub fn live_home(&self) -> &Path {
        &self.live_home
    }

    fn managed_files_static() -> Vec<String> {
        REQUIRED_MANAGED_FILES
            .into_iter()
            .chain(OPTIONAL_MANAGED_FILES)
            .map(ToOwned::to_owned)
            .collect()
    }

    fn validate_live_against(&self, sources: &ManagedSourceFiles) -> Result<(), RelayError> {
        ensure_same_contents(&sources.config, &self.live_home.join("config.toml"))?;

        if let Some(auth) = sources.auth.as_ref() {
            ensure_same_contents(auth, &self.live_home.join("auth.json"))?;
        } else {
            ensure_missing(&self.live_home.join("auth.json"))?;
        }

        if let Some(version) = sources.version.as_ref() {
            ensure_same_contents(version, &self.live_home.join("version.json"))?;
        } else {
            ensure_missing(&self.live_home.join("version.json"))?;
        }

        if let Some(binary) = find_binary("codex") {
            let output = Command::new(binary)
                .arg("--version")
                .output()
                .map_err(|error| RelayError::ExternalCommand(error.to_string()))?;
            if !output.status.success() {
                return Err(RelayError::ExternalCommand(
                    String::from_utf8_lossy(&output.stderr).trim().to_string(),
                ));
            }
        }

        Ok(())
    }

    fn sync_live_files(&self, sources: &ManagedSourceFiles) -> Result<(), RelayError> {
        copy_atomic(&sources.config, &self.live_home.join("config.toml"))?;
        sync_optional_file(sources.auth.as_deref(), &self.live_home.join("auth.json"))?;
        sync_optional_file(
            sources.version.as_deref(),
            &self.live_home.join("version.json"),
        )?;
        Ok(())
    }

    fn backup_live_files(&self, backup_dir: &Path) -> Result<Vec<String>, RelayError> {
        fs::create_dir_all(backup_dir)?;
        let managed = self.live_managed_files();
        let mut backups = Vec::new();

        if managed.config.exists() {
            let destination = backup_dir.join("config.toml");
            copy_atomic(&managed.config, &destination)?;
            backups.push(destination.to_string_lossy().into_owned());
        }
        if let Some(auth) = managed.auth.filter(|path| path.exists()) {
            let destination = backup_dir.join("auth.json");
            copy_atomic(&auth, &destination)?;
            backups.push(destination.to_string_lossy().into_owned());
        }
        if let Some(version) = managed.version.filter(|path| path.exists()) {
            let destination = backup_dir.join("version.json");
            copy_atomic(&version, &destination)?;
            backups.push(destination.to_string_lossy().into_owned());
        }

        Ok(backups)
    }

    fn restore_backup(&self, backup_dir: &Path) -> Result<(), RelayError> {
        let backup_config = backup_dir.join("config.toml");
        if backup_config.exists() {
            copy_atomic(&backup_config, &self.live_home.join("config.toml"))?;
        }

        let backup_auth = backup_dir.join("auth.json");
        if backup_auth.exists() {
            copy_atomic(&backup_auth, &self.live_home.join("auth.json"))?;
        } else {
            remove_if_exists(&self.live_home.join("auth.json"))?;
        }

        let backup_version = backup_dir.join("version.json");
        if backup_version.exists() {
            copy_atomic(&backup_version, &self.live_home.join("version.json"))?;
        } else {
            remove_if_exists(&self.live_home.join("version.json"))?;
        }

        Ok(())
    }

    fn live_managed_files(&self) -> ManagedSourceFiles {
        ManagedSourceFiles {
            config: self.live_home.join("config.toml"),
            auth: Some(self.live_home.join("auth.json")),
            version: Some(self.live_home.join("version.json")),
        }
    }

    fn resolve_sources(&self, profile: &Profile) -> Result<ManagedSourceFiles, RelayError> {
        let agent_home = profile.agent_home.as_ref().map(PathBuf::from);
        let config = profile
            .config_path
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| agent_home.as_ref().map(|path| path.join("config.toml")))
            .ok_or_else(|| {
                RelayError::Validation(
                    "profile must provide either config_path or agent_home/config.toml".into(),
                )
            })?;

        let auth = agent_home
            .as_ref()
            .map(|path| path.join("auth.json"))
            .filter(|path| path.exists());
        let version = agent_home
            .as_ref()
            .map(|path| path.join("version.json"))
            .filter(|path| path.exists());

        Ok(ManagedSourceFiles {
            config,
            auth,
            version,
        })
    }

    fn activate_profile(
        &self,
        profile: &Profile,
        snapshot_root: &Path,
    ) -> Result<SwitchCheckpoint, RelayError> {
        self.validate_profile(profile)?;

        let checkpoint_id = format!("ckpt_{}", Utc::now().timestamp_millis());
        let checkpoint_dir = snapshot_root.join(&checkpoint_id);
        let backup_dir = checkpoint_dir.join("live_backup");
        fs::create_dir_all(&backup_dir)?;

        let sources = self.resolve_sources(profile)?;
        let backup_paths = self.backup_live_files(&backup_dir)?;

        let attempt = (|| {
            self.sync_live_files(&sources)?;
            self.validate_live_against(&sources)
        })();

        if let Err(error) = attempt {
            self.restore_backup(&backup_dir)?;
            return Err(error);
        }

        Ok(SwitchCheckpoint {
            checkpoint_id,
            backup_paths,
            created_at: Utc::now(),
        })
    }
}

#[async_trait(?Send)]
impl AgentAdapter for CodexAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn binary_name(&self) -> &'static str {
        "codex"
    }

    fn home_env_var_name(&self) -> Option<&'static str> {
        Some("CODEX_HOME")
    }

    fn default_home(&self) -> Option<PathBuf> {
        default_codex_home()
    }

    fn live_home(&self) -> &Path {
        &self.live_home
    }

    fn managed_files(&self) -> Vec<String> {
        Self::managed_files_static()
    }

    fn validate_profile(&self, profile: &Profile) -> Result<(), RelayError> {
        let sources = self.resolve_sources(profile)?;
        if !sources.config.exists() {
            return Err(RelayError::Validation(format!(
                "profile config path does not exist: {}",
                sources.config.display()
            )));
        }
        if let Some(home) = profile.agent_home.as_ref() {
            let path = PathBuf::from(home);
            if !path.exists() || !path.is_dir() {
                return Err(RelayError::Validation(format!(
                    "profile agent home is not a directory: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }

    fn import_live_profile(&self, destination: &Path) -> Result<Vec<String>, RelayError> {
        fs::create_dir_all(destination)?;
        let managed = self.live_managed_files();
        if !managed.config.exists() {
            return Err(RelayError::Validation(format!(
                "live Codex config not found: {}",
                managed.config.display()
            )));
        }

        let mut copied = Vec::new();
        copy_atomic(&managed.config, &destination.join("config.toml"))?;
        copied.push("config.toml".to_string());

        if let Some(auth) = managed.auth.filter(|path| path.exists()) {
            copy_atomic(&auth, &destination.join("auth.json"))?;
            copied.push("auth.json".to_string());
        }

        if let Some(version) = managed.version.filter(|path| path.exists()) {
            copy_atomic(&version, &destination.join("version.json"))?;
            copied.push("version.json".to_string());
        }

        Ok(copied)
    }

    async fn import_profile(
        &self,
        store: &SqliteStore,
        paths: &RelayPaths,
        nickname: Option<String>,
        priority: i32,
    ) -> Result<Profile, RelayError> {
        login::import_profile(self, store, paths, nickname, priority).await
    }

    async fn login_profile(
        &self,
        store: &SqliteStore,
        profiles_dir: &Path,
        nickname: Option<String>,
        priority: i32,
        mode: AgentLoginMode,
        cancel_requested: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<AgentLinkResult, RelayError> {
        login::login_profile(
            self,
            store,
            profiles_dir,
            nickname,
            priority,
            mode,
            cancel_requested,
        )
        .await
    }

    async fn relink_profile(
        &self,
        store: &SqliteStore,
        profile: &Profile,
    ) -> Result<ProfileProbeIdentity, RelayError> {
        login::relink_profile(self, store, profile).await
    }

    async fn recover_profiles(
        &self,
        store: &SqliteStore,
        paths: &RelayPaths,
    ) -> Result<ProfileRecoveryReport, RelayError> {
        recover::recover_profiles(self, store, paths).await
    }

    fn activate(
        &self,
        profile: &Profile,
        snapshot_root: &Path,
    ) -> Result<SwitchCheckpoint, RelayError> {
        self.activate_profile(profile, snapshot_root)
    }

    fn rollback_checkpoint(
        &self,
        snapshot_root: &Path,
        checkpoint_id: &str,
    ) -> Result<(), RelayError> {
        let backup_dir = snapshot_root.join(checkpoint_id).join("live_backup");
        self.restore_backup(&backup_dir)
    }
}

fn default_codex_home() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".codex"))
}

fn live_codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(default_codex_home)
}

#[async_trait(?Send)]
impl UsageProvider for CodexAdapter {
    fn collect_local_usage(
        &self,
        target_profile: Option<&Profile>,
        active_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError> {
        usage::collect_local(self, target_profile, active_profile)
    }

    async fn collect_remote_usage(
        &self,
        store: &SqliteStore,
        target_profile: Option<&Profile>,
    ) -> Result<Option<UsageSnapshot>, RelayError> {
        usage::collect_remote(store, target_profile).await
    }
}

fn ensure_same_contents(source: &Path, destination: &Path) -> Result<(), RelayError> {
    let source_bytes = fs::read(source)?;
    let destination_bytes = fs::read(destination)?;
    if source_bytes != destination_bytes {
        return Err(RelayError::Validation(format!(
            "post-switch validation mismatch for {}",
            destination.display()
        )));
    }
    Ok(())
}

fn ensure_missing(path: &Path) -> Result<(), RelayError> {
    if path.exists() {
        return Err(RelayError::Validation(format!(
            "post-switch validation expected {} to be absent",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn copy_atomic(source: &Path, destination: &Path) -> Result<(), RelayError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = destination.with_extension("tmp");
    fs::copy(source, &temp)?;
    fs::rename(temp, destination)?;
    Ok(())
}

fn sync_optional_file(source: Option<&Path>, destination: &Path) -> Result<(), RelayError> {
    match source {
        Some(source) => copy_atomic(source, destination),
        None => remove_if_exists(destination),
    }
}

fn remove_if_exists(path: &Path) -> Result<(), RelayError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}
