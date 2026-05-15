use camino::{Utf8Path, Utf8PathBuf};

use crate::WorkspaceError;

pub(crate) fn replace_file(temp: &Utf8Path, path: &Utf8Path) -> Result<(), WorkspaceError> {
    match std::fs::rename(temp, path) {
        Ok(()) => Ok(()),
        Err(source) if path.exists() => replace_existing_file(temp, path, source),
        Err(source) => Err(WorkspaceError::WriteFile {
            path: path.to_owned(),
            source,
        }),
    }
}

fn replace_existing_file(
    temp: &Utf8Path,
    path: &Utf8Path,
    original_error: std::io::Error,
) -> Result<(), WorkspaceError> {
    let backup = temp_path(path, format!("backup-{}", crate::lock::unix_ms()));
    std::fs::rename(path, &backup).map_err(|source| WorkspaceError::WriteFile {
        path: path.to_owned(),
        source,
    })?;
    match std::fs::rename(temp, path) {
        Ok(()) => {
            let _ = std::fs::remove_file(&backup);
            Ok(())
        }
        Err(source) => {
            let _ = std::fs::rename(&backup, path);
            Err(WorkspaceError::WriteFile {
                path: path.to_owned(),
                source: if source.kind() == std::io::ErrorKind::AlreadyExists {
                    original_error
                } else {
                    source
                },
            })
        }
    }
}

pub(crate) fn temp_path(path: &Utf8Path, suffix: impl AsRef<str>) -> Utf8PathBuf {
    let file_name = path.file_name().unwrap_or("workspace");
    path.with_file_name(format!(
        "{file_name}.tmp-{}-{}",
        std::process::id(),
        suffix.as_ref()
    ))
}
