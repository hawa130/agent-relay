use crate::adapters::AdapterRegistry;
use crate::models::{DoctorReport, RelayError};
use crate::platform::{RelayPaths, find_binary};

pub fn run(paths: &RelayPaths) -> Result<DoctorReport, RelayError> {
    let adapters = AdapterRegistry::new()?;
    let primary = adapters.primary();
    let agent_binary =
        find_binary(primary.binary_name()).map(|path| path.to_string_lossy().into_owned());
    let default_agent_home = primary.default_home();
    let live_agent_home = primary.live_home().to_string_lossy().into_owned();

    Ok(DoctorReport {
        platform: std::env::consts::OS.to_string(),
        relay_home: paths.relay_home.to_string_lossy().into_owned(),
        relay_db_path: paths.db_path.to_string_lossy().into_owned(),
        relay_log_path: paths.log_file.to_string_lossy().into_owned(),
        primary_agent: adapters.primary_kind(),
        agent_home_env_name: primary.home_env_var_name().map(str::to_owned),
        agent_home_env_value: primary
            .home_env_var_name()
            .and_then(|name| std::env::var(name).ok()),
        live_agent_home,
        agent_binary,
        default_agent_home: default_agent_home
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        default_agent_home_exists: default_agent_home
            .as_ref()
            .is_some_and(|path| path.exists()),
        managed_files: primary.managed_files(),
    })
}
