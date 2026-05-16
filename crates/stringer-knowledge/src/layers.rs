use std::fs;
use std::io::{BufRead, BufReader};

use camino::{Utf8Path, Utf8PathBuf};
use stringer_pipeline::KnowledgeLayer;

use crate::KnowledgeError;
use crate::index::{
    KnowledgeFileKind, KnowledgeSourceFile, knowledge_index_path, source_file_from_path,
};
use stringer_workspace_core::WorkspaceCoreError;
use stringer_workspace_core::WorkspaceSettings;

pub(crate) const GLOBAL_LAYER: &str = "global";
pub(crate) const WORKSPACE_LAYER: &str = "workspace";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnowledgeLayerKind {
    Global,
    Workspace,
}

impl KnowledgeLayerKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Global => GLOBAL_LAYER,
            Self::Workspace => WORKSPACE_LAYER,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnowledgeLayerSelection {
    All,
    WorkspaceOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeResourceLayer {
    pub(crate) kind: KnowledgeLayerKind,
    pub(crate) index_path: Utf8PathBuf,
    pub(crate) files: Vec<KnowledgeSourceFile>,
}

impl KnowledgeResourceLayer {
    pub(crate) fn name(&self) -> &'static str {
        self.kind.name()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct KnowledgeResourceSet {
    pub(crate) layers: Vec<KnowledgeResourceLayer>,
}

impl KnowledgeResourceSet {
    pub(crate) fn all_source_files(&self) -> Vec<KnowledgeSourceFile> {
        self.layers
            .iter()
            .flat_map(|layer| layer.files.iter().cloned())
            .collect()
    }
}

pub(crate) fn collect_knowledge_resources(
    workspace: &Utf8Path,
    settings: &WorkspaceSettings,
    selection: KnowledgeLayerSelection,
) -> Result<KnowledgeResourceSet, KnowledgeError> {
    let workspace_knowledge_root = workspace.join("knowledge");
    let mut layers = Vec::new();
    if selection == KnowledgeLayerSelection::All
        && let Some(global_root) = settings.global_knowledge_root.clone()
        && !same_path(&global_root, &workspace_knowledge_root)
    {
        let layer = collect_layer(KnowledgeLayerKind::Global, &global_root)?;
        if layer_has_declared_entries(&layer)? {
            layers.push(layer);
        }
    }

    let workspace_layer = collect_workspace_layer(workspace)?;
    if selection == KnowledgeLayerSelection::WorkspaceOnly
        || layer_has_declared_entries(&workspace_layer)?
    {
        layers.push(workspace_layer);
    }
    Ok(KnowledgeResourceSet { layers })
}

pub(crate) fn collect_workspace_layer(
    workspace: &Utf8Path,
) -> Result<KnowledgeResourceLayer, KnowledgeError> {
    let workspace_knowledge_root = workspace.join("knowledge");
    let mut layer = collect_layer(KnowledgeLayerKind::Workspace, &workspace_knowledge_root)?;
    layer.index_path = knowledge_index_path(workspace);
    Ok(layer)
}

fn collect_layer(
    kind: KnowledgeLayerKind,
    root: &Utf8Path,
) -> Result<KnowledgeResourceLayer, KnowledgeError> {
    Ok(KnowledgeResourceLayer {
        kind,
        files: collect_files_for_layer(kind.name(), root)?,
        index_path: root.join("index.sqlite"),
    })
}

fn layer_has_declared_entries(layer: &KnowledgeResourceLayer) -> Result<bool, KnowledgeError> {
    for file in &layer.files {
        if source_file_has_declared_entries(file)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn source_file_has_declared_entries(file: &KnowledgeSourceFile) -> Result<bool, KnowledgeError> {
    match file.kind {
        KnowledgeFileKind::Memory => memory_file_has_declared_entries(&file.path),
        KnowledgeFileKind::Terms | KnowledgeFileKind::Rules => toml_file_has_declared_entries(file),
    }
}

fn memory_file_has_declared_entries(path: &Utf8Path) -> Result<bool, KnowledgeError> {
    let file = fs::File::open(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|source| WorkspaceCoreError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        if !line.trim().is_empty() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn toml_file_has_declared_entries(file: &KnowledgeSourceFile) -> Result<bool, KnowledgeError> {
    let text = fs::read_to_string(&file.path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: file.path.clone(),
        source,
    })?;
    let mut layer = KnowledgeLayer::new(&file.layer);
    match file.kind {
        KnowledgeFileKind::Terms => layer.add_terms_toml(file.path.as_str(), &text)?,
        KnowledgeFileKind::Rules => layer.add_rules_toml(file.path.as_str(), &text)?,
        KnowledgeFileKind::Memory => unreachable!("memory files are checked without TOML parsing"),
    }
    Ok(!layer.is_empty())
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
