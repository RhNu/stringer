use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::WorkspaceCoreError;

const LOCK_FILE: &str = "lock";

#[derive(Debug)]
pub struct WorkspaceLock {
    path: Utf8PathBuf,
}

#[derive(Debug, Serialize)]
struct LockMetadata {
    pid: u32,
    created_at_unix_ms: u128,
}

impl WorkspaceLock {
    pub fn acquire(root: &Utf8Path) -> Result<Self, WorkspaceCoreError> {
        fs::create_dir_all(root).map_err(|source| WorkspaceCoreError::WriteFile {
            path: root.to_owned(),
            source,
        })?;
        let path = root.join(LOCK_FILE);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    WorkspaceCoreError::WorkspaceLocked { path: path.clone() }
                } else {
                    WorkspaceCoreError::WriteFile {
                        path: path.clone(),
                        source,
                    }
                }
            })?;
        let lock = Self { path };
        let metadata = LockMetadata {
            pid: std::process::id(),
            created_at_unix_ms: unix_ms(),
        };
        serde_json::to_writer(&mut file, &metadata).map_err(|source| WorkspaceCoreError::Json {
            path: lock.path.clone(),
            source,
        })?;
        file.write_all(b"\n")
            .map_err(|source| WorkspaceCoreError::WriteFile {
                path: lock.path.clone(),
                source,
            })?;
        file.flush()
            .map_err(|source| WorkspaceCoreError::WriteFile {
                path: lock.path.clone(),
                source,
            })?;
        Ok(lock)
    }
}

impl Drop for WorkspaceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
