use std::fs;

use camino::{Utf8Path, Utf8PathBuf};

use crate::KnowledgeError;
use crate::index::{
    KnowledgeFileKind, KnowledgeSourceFile, knowledge_index_path, source_file_from_path,
};
use stringer_workspace_core::WorkspaceCoreError;
use stringer_workspace_core::WorkspaceSettings;

pub(crate) const GLOBAL_LAYER: &str = "global";
pub(crate) const WORKSPACE_LAYER: &str = "workspace";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeIndexLayer {
    pub(crate) name: String,
    pub(crate) index_path: Utf8PathBuf,
    pub(crate) files: Vec<KnowledgeSourceFile>,
}

pub(crate) fn collect_index_layers(
    workspace: &Utf8Path,
    settings: &WorkspaceSettings,
) -> Result<Vec<KnowledgeIndexLayer>, KnowledgeError> {
    let workspace_knowledge_root = workspace.join("knowledge");
    let mut layers = Vec::new();
    if let Some(global_root) = settings.global_knowledge_root.clone()
        && !same_path(&global_root, &workspace_knowledge_root)
    {
        let files = collect_files_for_layer(GLOBAL_LAYER, &global_root)?;
        if !files.is_empty() {
            layers.push(KnowledgeIndexLayer {
                name: GLOBAL_LAYER.to_string(),
                index_path: global_root.join("index.sqlite"),
                files,
            });
        }
    }
    layers.push(collect_workspace_index_layer(workspace)?);
    Ok(layers)
}

pub(crate) fn collect_workspace_index_layer(
    workspace: &Utf8Path,
) -> Result<KnowledgeIndexLayer, KnowledgeError> {
    let workspace_knowledge_root = workspace.join("knowledge");
    Ok(KnowledgeIndexLayer {
        name: WORKSPACE_LAYER.to_string(),
        files: collect_files_for_layer(WORKSPACE_LAYER, &workspace_knowledge_root)?,
        index_path: knowledge_index_path(workspace),
    })
}

pub(crate) fn collect_all_source_files(layers: &[KnowledgeIndexLayer]) -> Vec<KnowledgeSourceFile> {
    layers
        .iter()
        .flat_map(|layer| layer.files.iter().cloned())
        .collect()
}

fn collect_files_for_layer(
    layer: &str,
    root: &Utf8Path,
) -> Result<Vec<KnowledgeSourceFile>, KnowledgeError> {
    let mut files = Vec::new();
    for (kind, folder, extension) in [
        (KnowledgeFileKind::Terms, "terms", "toml"),
        (KnowledgeFileKind::Memory, "memory", "jsonl"),
        (KnowledgeFileKind::Rules, "rules", "toml"),
    ] {
        for path in sorted_files(&root.join(folder), extension)? {
            files.push(source_file_from_path(path, layer, kind)?);
        }
    }
    Ok(files)
}

fn sorted_files(root: &Utf8Path, extension: &str) -> Result<Vec<Utf8PathBuf>, KnowledgeError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_sorted_files(root, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_sorted_files(
    root: &Utf8Path,
    extension: &str,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), KnowledgeError> {
    for entry in fs::read_dir(root).map_err(|source| WorkspaceCoreError::ReadFile {
        path: root.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceCoreError::ReadFile {
            path: root.to_owned(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceCoreError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "knowledge file path is not valid UTF-8".to_string(),
            }
        })?;
        if path.is_dir() {
            collect_sorted_files(&path, extension, files)?;
        } else if path.extension() == Some(extension) {
            files.push(path);
        }
    }
    Ok(())
}

fn same_path(left: &Utf8Path, right: &Utf8Path) -> bool {
    left.as_str().replace('\\', "/").to_lowercase()
        == right.as_str().replace('\\', "/").to_lowercase()
}
