use std::env;
use std::path::PathBuf;

pub fn find_binary(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;

    env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|candidate| candidate.is_file())
}

pub fn default_codex_home() -> Option<PathBuf> {
    dirs::home_dir().map(|path| path.join(".codex"))
}

pub fn live_codex_home() -> Option<PathBuf> {
    env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(default_codex_home)
}
