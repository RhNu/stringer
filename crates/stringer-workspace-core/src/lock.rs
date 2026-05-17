use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;

use crate::WorkspaceCoreError;

const LOCK_FILE: &str = "lock";
const DEFAULT_LOCK_WAIT: Duration = Duration::from_secs(30);
const DEFAULT_LOCK_POLL: Duration = Duration::from_millis(50);

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
        Self::acquire_with_timeout(root, DEFAULT_LOCK_WAIT, DEFAULT_LOCK_POLL)
    }

    fn acquire_with_timeout(
        root: &Utf8Path,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<Self, WorkspaceCoreError> {
        let started = Instant::now();
        loop {
            match Self::try_acquire(root) {
                Ok(lock) => return Ok(lock),
                Err(WorkspaceCoreError::WorkspaceLocked { path }) => {
                    let elapsed = started.elapsed();
                    if elapsed >= timeout {
                        return Err(WorkspaceCoreError::WorkspaceLocked { path });
                    }
                    let sleep_for = poll_interval.min(timeout.saturating_sub(elapsed));
                    if !sleep_for.is_zero() {
                        thread::sleep(sleep_for);
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn try_acquire(root: &Utf8Path) -> Result<Self, WorkspaceCoreError> {
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

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use camino::Utf8PathBuf;

    use super::*;

    #[test]
    fn lock_waits_until_existing_lock_is_released() {
        let root = temp_lock_root("waits");
        let first = WorkspaceLock::acquire(&root).unwrap();
        let released = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            drop(first);
        });

        let second = WorkspaceLock::acquire_with_timeout(
            &root,
            Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .unwrap();

        drop(second);
        released.join().unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lock_returns_locked_after_wait_timeout() {
        let root = temp_lock_root("timeout");
        let first = WorkspaceLock::acquire(&root).unwrap();

        let error = WorkspaceLock::acquire_with_timeout(
            &root,
            Duration::from_millis(20),
            Duration::from_millis(5),
        )
        .unwrap_err();

        assert!(matches!(error, WorkspaceCoreError::WorkspaceLocked { .. }));
        drop(first);
        let _ = fs::remove_dir_all(root);
    }

    fn temp_lock_root(label: &str) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(std::env::temp_dir().join(format!(
            "stringer-lock-{label}-{}-{}",
            std::process::id(),
            unix_ms()
        )))
        .unwrap()
    }
}
