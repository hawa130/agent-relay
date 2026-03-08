use crate::models::{LogTail, RelayError};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileLogStore {
    path: PathBuf,
}

impl FileLogStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append(
        &self,
        level: &str,
        event: &str,
        message: impl AsRef<str>,
    ) -> Result<(), RelayError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(
            file,
            "{} {} {} {}",
            Utc::now().to_rfc3339(),
            level.to_uppercase(),
            event,
            message.as_ref()
        )?;
        Ok(())
    }

    pub fn tail(&self, lines: usize) -> Result<LogTail, RelayError> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };

        let collected = contents
            .lines()
            .rev()
            .take(lines)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        Ok(LogTail {
            path: self.path.to_string_lossy().into_owned(),
            lines: collected,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
