use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use stringer_core::FileRole;

use crate::ReaderError;

pub(crate) fn logical_path_for_root_relative(
    root: &Utf8Path,
    path: &Utf8Path,
) -> Result<Utf8PathBuf, ReaderError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|source| ReaderError::Archive {
            path: path.to_owned(),
            message: source.to_string(),
        })?;
    logical_path_from_virtual_path(relative).ok_or_else(|| ReaderError::Archive {
        path: path.to_owned(),
        message: "path contains invalid virtual path components".to_string(),
    })
}

pub(crate) fn logical_path_for_archive_entry(entry_path: &Utf8Path) -> Option<Utf8PathBuf> {
    logical_path_from_virtual_path(entry_path)
}

fn logical_path_from_virtual_path(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut parts = Vec::new();
    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Utf8Component::Normal(part) => {
                if !saw_normal && part.eq_ignore_ascii_case("Data") {
                    saw_normal = true;
                    continue;
                }
                saw_normal = true;
                parts.push(part);
            }
            Utf8Component::Prefix(_)
            | Utf8Component::RootDir
            | Utf8Component::CurDir
            | Utf8Component::ParentDir => return None,
        }
    }

    if parts.is_empty() {
        return None;
    }

    let mut logical_path = Utf8PathBuf::from("Data");
    for part in parts {
        logical_path.push(part);
    }
    Some(logical_path)
}

pub(crate) fn normalize_lookup_str(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub(crate) fn is_archive_path(path: &Utf8Path) -> bool {
    matches!(
        path.extension()
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("bsa" | "ba2")
    )
}

pub(crate) fn is_known_text_asset(path: &Utf8Path) -> bool {
    matches!(
        FileRole::from_format(stringer_core::FileFormat::from_path(path)),
        FileRole::Plugin | FileRole::Strings | FileRole::Pex | FileRole::Scaleform
    )
}

pub(crate) fn utf8_path(path: std::path::PathBuf) -> Result<Utf8PathBuf, ReaderError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| ReaderError::Archive {
        path: Utf8PathBuf::from(path.to_string_lossy().as_ref()),
        message: "path is not valid UTF-8".to_string(),
    })
}
