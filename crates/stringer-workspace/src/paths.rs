use std::fs;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use stringer_core::{FileAsset, FileBundle};

use crate::WorkspaceError;

pub fn write_output_assets(root: &Utf8Path, assets: &[FileAsset]) -> Result<usize, WorkspaceError> {
    for asset in assets {
        let target = output_path(root, asset.path())?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
                path: parent.to_owned(),
                source,
            })?;
        }
        fs::write(&target, asset.bytes()).map_err(|source| WorkspaceError::WriteFile {
            path: target,
            source,
        })?;
    }
    Ok(assets.len())
}

pub fn ensure_output_outside_source(
    source_root: &Utf8Path,
    output_root: &Utf8Path,
) -> Result<(), WorkspaceError> {
    let source = normalized_absolute_components(source_root)?;
    let output = normalized_absolute_components(output_root)?;
    if path_starts_with(&output, &source) {
        return Err(WorkspaceError::InvalidOutputRoot {
            root: output_root.to_owned(),
            message: "output must be outside the source root".to_string(),
        });
    }
    Ok(())
}

pub fn ensure_workspace_outside_source(
    source_root: &Utf8Path,
    workspace: &Utf8Path,
) -> Result<(), WorkspaceError> {
    let source = normalized_absolute_components(source_root)?;
    let workspace_components = normalized_absolute_components(workspace)?;
    if path_starts_with(&workspace_components, &source) {
        return Err(WorkspaceError::InvalidTranslationPackagePath {
            path: workspace.to_string(),
            message: "workspace must be outside the source root".to_string(),
        });
    }
    Ok(())
}

pub fn changed_assets(
    original: &FileBundle,
    assets: impl IntoIterator<Item = FileAsset>,
) -> Result<Vec<FileAsset>, WorkspaceError> {
    let mut output = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for asset in assets {
        let key = normalize_path(asset.path().as_str());
        if !seen.insert(key) {
            return Err(WorkspaceError::DuplicateOutputPath {
                path: asset.path().to_string(),
            });
        }
        let changed = original
            .get(asset.path().as_str())
            .is_none_or(|input| input.bytes() != asset.bytes());
        if changed {
            output.push(asset);
        }
    }
    Ok(output)
}

fn output_path(root: &Utf8Path, logical_path: &Utf8Path) -> Result<Utf8PathBuf, WorkspaceError> {
    let path_text = logical_path.to_string();
    let mut components = logical_path.components();
    let Some(Utf8Component::Normal(first)) = components.next() else {
        return Err(invalid_path(path_text, "path must start with Data"));
    };
    if !first.eq_ignore_ascii_case("Data") {
        return Err(invalid_path(path_text, "path must start with Data"));
    }

    let mut output = root.join(first);
    for component in components {
        let Utf8Component::Normal(part) = component else {
            return Err(invalid_path(
                logical_path.to_string(),
                "path must not contain absolute, current, or parent components",
            ));
        };
        output.push(part);
    }
    Ok(output)
}

fn invalid_path(path: String, message: impl Into<String>) -> WorkspaceError {
    WorkspaceError::InvalidLogicalPath {
        path,
        message: message.into(),
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn normalized_absolute_components(path: &Utf8Path) -> Result<Vec<String>, WorkspaceError> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        let current = std::env::current_dir()
            .map_err(|source| WorkspaceError::CurrentDirectory { source })?;
        Utf8PathBuf::from_path_buf(current)
            .map_err(|path| WorkspaceError::InvalidOutputRoot {
                root: Utf8PathBuf::from(path.to_string_lossy().as_ref()),
                message: "current directory is not valid UTF-8".to_string(),
            })?
            .join(path)
    };

    let mut parts = Vec::new();
    for component in absolute.components() {
        match component {
            Utf8Component::CurDir => {}
            Utf8Component::ParentDir => {
                parts.pop();
            }
            _ => parts.push(component.as_str().to_ascii_lowercase()),
        }
    }
    Ok(parts)
}

fn path_starts_with(path: &[String], prefix: &[String]) -> bool {
    path.len() >= prefix.len() && path.iter().zip(prefix).all(|(left, right)| left == right)
}
