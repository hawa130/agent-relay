use super::CodexAdapter;
use super::auth::load_probe_identity_from_home;
use crate::models::{
    AgentKind, AuthMode, ProfileProbeIdentity, ProfileRecoveryReport, RecoveredProfile, RelayError,
    SkippedRecoveredProfile,
};
use crate::platform::RelayPaths;
use crate::services::profile_service;
use crate::store::{AddProfileRecord, SqliteStore};
use chrono::Utc;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_RECOVERY_PRIORITY: i32 = 100;

pub(crate) async fn recover_profiles(
    adapter: &CodexAdapter,
    store: &SqliteStore,
    paths: &RelayPaths,
) -> Result<ProfileRecoveryReport, RelayError> {
    let mut existing_homes: HashSet<String> = store
        .list_profiles()
        .await?
        .into_iter()
        .filter_map(|profile| profile.agent_home)
        .collect();
    let mut recovered = Vec::new();
    let mut skipped = Vec::new();

    let mut entries: Vec<PathBuf> = fs::read_dir(&paths.profiles_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    entries.sort();
    let scanned_dirs = entries.len();

    for (index, path) in entries.into_iter().enumerate() {
        let source_dir = path.to_string_lossy().into_owned();
        let config_path = path.join("config.toml");

        if !config_path.exists() {
            skipped.push(SkippedRecoveredProfile {
                source_dir,
                reason: "missing config.toml".into(),
            });
            continue;
        }

        if existing_homes.contains(&source_dir) {
            skipped.push(SkippedRecoveredProfile {
                source_dir,
                reason: "profile already exists for this agent home".into(),
            });
            continue;
        }

        let identity = load_probe_identity_from_home("pending", &path).ok();
        let nickname = recovered_nickname(&path, identity.as_ref());
        let priority = DEFAULT_RECOVERY_PRIORITY + index as i32;

        match profile_service::add_profile(
            store,
            adapter,
            AddProfileRecord {
                agent: AgentKind::Codex,
                nickname,
                priority,
                config_path: Some(config_path),
                agent_home: Some(path.clone()),
                auth_mode: AuthMode::ConfigFilesystem,
            },
        )
        .await
        {
            Ok(profile) => {
                let probe_identity_restored =
                    restore_probe_identity(store, &profile.id, identity).await;
                existing_homes.insert(source_dir.clone());
                recovered.push(RecoveredProfile {
                    source_dir,
                    profile,
                    probe_identity_restored,
                });
            }
            Err(error) => {
                skipped.push(SkippedRecoveredProfile {
                    source_dir,
                    reason: error.to_string(),
                });
            }
        }
    }

    Ok(ProfileRecoveryReport {
        scanned_dirs,
        recovered,
        skipped,
    })
}

async fn restore_probe_identity(
    store: &SqliteStore,
    profile_id: &str,
    identity: Option<ProfileProbeIdentity>,
) -> bool {
    let Some(identity) = identity else {
        return false;
    };
    store
        .upsert_probe_identity(&ProfileProbeIdentity {
            profile_id: profile_id.to_string(),
            ..identity
        })
        .await
        .is_ok()
}

fn recovered_nickname(path: &Path, identity: Option<&ProfileProbeIdentity>) -> String {
    identity
        .and_then(|identity| identity.email().map(ToOwned::to_owned))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| {
                    format!("Recovered Codex {}", Utc::now().format("%Y%m%d-%H%M%S"))
                })
        })
}
