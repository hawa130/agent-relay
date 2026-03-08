use crate::models::RelayError;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RelayPaths {
    pub relay_home: PathBuf,
    pub db_path: PathBuf,
    pub state_path: PathBuf,
    pub usage_path: PathBuf,
    pub logs_dir: PathBuf,
    pub log_file: PathBuf,
    pub profiles_dir: PathBuf,
    pub snapshots_dir: PathBuf,
    pub exports_dir: PathBuf,
}

impl RelayPaths {
    pub fn from_env() -> Result<Self, RelayError> {
        let relay_home = match std::env::var_os("RELAY_HOME") {
            Some(value) => PathBuf::from(value),
            None => dirs::home_dir()
                .map(|path| path.join(".relay"))
                .ok_or_else(|| RelayError::Internal("failed to resolve home directory".into()))?,
        };

        Ok(Self::from_root(relay_home))
    }

    pub fn from_root(relay_home: PathBuf) -> Self {
        Self {
            db_path: relay_home.join("relay.db"),
            state_path: relay_home.join("state.json"),
            usage_path: relay_home.join("usage.json"),
            logs_dir: relay_home.join("logs"),
            log_file: relay_home.join("logs").join("relay.log"),
            profiles_dir: relay_home.join("profiles"),
            snapshots_dir: relay_home.join("snapshots"),
            exports_dir: relay_home.join("exports"),
            relay_home,
        }
    }

    pub fn ensure_layout(&self) -> Result<(), RelayError> {
        fs::create_dir_all(&self.relay_home)?;
        fs::create_dir_all(&self.logs_dir)?;
        fs::create_dir_all(&self.profiles_dir)?;
        fs::create_dir_all(&self.snapshots_dir)?;
        fs::create_dir_all(&self.exports_dir)?;
        Ok(())
    }
}
