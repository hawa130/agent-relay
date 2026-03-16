use crate::models::{LogTail, RelayError};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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

    pub async fn append(
        &self,
        level: String,
        event: String,
        message: String,
    ) -> Result<(), RelayError> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.append_sync(&level, &event, &message))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    pub async fn tail(&self, lines: usize) -> Result<LogTail, RelayError> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || store.tail_sync(lines))
            .await
            .map_err(|e| RelayError::Internal(format!("blocking task failed: {e}")))?
    }

    fn append_sync(&self, level: &str, event: &str, message: &str) -> Result<(), RelayError> {
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
            message
        )?;
        Ok(())
    }

    fn tail_sync(&self, lines: usize) -> Result<LogTail, RelayError> {
        const TAIL_CHUNK_SIZE: u64 = 64 * 1024;

        let mut file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LogTail {
                    path: self.path.to_string_lossy().into_owned(),
                    lines: Vec::new(),
                });
            }
            Err(error) => return Err(error.into()),
        };

        let file_len = file.metadata()?.len();
        let read_from = file_len.saturating_sub(TAIL_CHUNK_SIZE);
        file.seek(SeekFrom::Start(read_from))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // If we started mid-file, skip the first partial line
        if read_from > 0 {
            if let Some(pos) = contents.find('\n') {
                contents = contents[pos + 1..].to_string();
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn append_creates_file_and_writes_line() {
        let temp = tempdir().expect("tempdir");
        let store = FileLogStore::new(temp.path().join("test.log"));

        store
            .append("INFO".into(), "startup".into(), "hello world".into())
            .await
            .expect("append");

        let tail = store.tail(10).await.expect("tail");
        assert_eq!(tail.lines.len(), 1);
        assert!(tail.lines[0].contains("INFO"));
        assert!(tail.lines[0].contains("startup"));
        assert!(tail.lines[0].contains("hello world"));
    }

    #[tokio::test]
    async fn tail_returns_last_n_lines() {
        let temp = tempdir().expect("tempdir");
        let store = FileLogStore::new(temp.path().join("test.log"));

        for i in 0..10 {
            store
                .append("INFO".into(), "event".into(), format!("line {i}"))
                .await
                .expect("append");
        }

        let tail = store.tail(3).await.expect("tail");
        assert_eq!(tail.lines.len(), 3);
        assert!(tail.lines[0].contains("line 7"));
        assert!(tail.lines[1].contains("line 8"));
        assert!(tail.lines[2].contains("line 9"));
    }

    #[tokio::test]
    async fn tail_on_missing_file_returns_empty() {
        let temp = tempdir().expect("tempdir");
        let store = FileLogStore::new(temp.path().join("nonexistent.log"));

        let tail = store.tail(10).await.expect("tail");

        assert!(tail.lines.is_empty());
    }

    #[tokio::test]
    async fn tail_with_fewer_lines_than_requested() {
        let temp = tempdir().expect("tempdir");
        let store = FileLogStore::new(temp.path().join("test.log"));

        store
            .append("WARN".into(), "test".into(), "only one".into())
            .await
            .expect("append");

        let tail = store.tail(100).await.expect("tail");
        assert_eq!(tail.lines.len(), 1);
    }

    #[tokio::test]
    async fn multiple_appends_accumulate() {
        let temp = tempdir().expect("tempdir");
        let store = FileLogStore::new(temp.path().join("test.log"));

        store
            .append("INFO".into(), "a".into(), "first".into())
            .await
            .expect("append 1");
        store
            .append("ERROR".into(), "b".into(), "second".into())
            .await
            .expect("append 2");
        store
            .append("DEBUG".into(), "c".into(), "third".into())
            .await
            .expect("append 3");

        let tail = store.tail(10).await.expect("tail");
        assert_eq!(tail.lines.len(), 3);
        assert!(tail.lines[0].contains("first"));
        assert!(tail.lines[2].contains("third"));
    }
}
