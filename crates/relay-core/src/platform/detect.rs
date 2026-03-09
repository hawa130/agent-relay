use std::env;
use std::path::PathBuf;

pub fn find_binary(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;

    env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|candidate| candidate.is_file())
}
