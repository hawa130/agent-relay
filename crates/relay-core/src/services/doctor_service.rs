use crate::adapters::CodexAdapter;
use crate::models::{DoctorReport, RelayError};
use crate::platform::{RelayPaths, default_codex_home, find_binary};

pub fn run(paths: &RelayPaths) -> Result<DoctorReport, RelayError> {
    let codex_binary = find_binary("codex").map(|path| path.to_string_lossy().into_owned());
    let codex_home = default_codex_home();
    let live_codex_home = CodexAdapter::new()?
        .live_home()
        .to_string_lossy()
        .into_owned();

    Ok(DoctorReport {
        platform: std::env::consts::OS.to_string(),
        relay_home: paths.relay_home.to_string_lossy().into_owned(),
        relay_db_path: paths.db_path.to_string_lossy().into_owned(),
        relay_log_path: paths.log_file.to_string_lossy().into_owned(),
        live_codex_home,
        codex_binary,
        codex_home: codex_home
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        codex_home_exists: codex_home.as_ref().is_some_and(|path| path.exists()),
        managed_files: CodexAdapter::managed_files(),
    })
}
