use std::collections::BTreeMap;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use stringer_core::{FileAsset, FileBundle, StringEntryBundle, StringEntryView};
use stringer_pex::{PexStringBundle, ReadPexOptions, read_pex_strings, write_pex_strings};
use stringer_plugin::{
    LocalizationBundle, ReadOptions, WriteOptions, read_localization, write_localization,
};
use stringer_reader::{ReadModOptions, read_mod_root};
use stringer_scaleform::{
    ScaleformTranslationBundle, read_scaleform_translations, write_scaleform_translations,
};
use stringer_workspace_core::{
    PackagedTranslationRecord, WorkspaceLock, WorkspaceSettings, external_entry_id,
    packaged_record_from_entry, read_translation_package, write_translation_package,
};
use stringer_workspace_ops::{CountBatchOptions, count_batch};
use tracing::{debug, info};

use crate::WorkspaceError;
use crate::paths::{
    changed_assets, ensure_output_outside_source, ensure_workspace_outside_source,
    write_output_assets,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTranslationsOptions {
    pub source_root: Utf8PathBuf,
    pub workspace: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTranslationsOptions {
    pub workspace: Utf8PathBuf,
    pub source_root: Option<Utf8PathBuf>,
    pub output: Utf8PathBuf,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportSummary {
    pub entries: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub applied_entries: usize,
    pub written_files: usize,
}

enum EditableBundle {
    Plugin(LocalizationBundle),
    Pex(PexStringBundle),
    Scaleform(ScaleformTranslationBundle),
}

impl EditableBundle {
    fn collect_records(
        &self,
        settings: &WorkspaceSettings,
        records: &mut Vec<PackagedTranslationRecord>,
    ) {
        match self {
            Self::Plugin(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
            Self::Pex(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
            Self::Scaleform(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
        }
    }

    fn apply_translations(&mut self, translations: &mut BTreeMap<String, String>) -> usize {
        match self {
            Self::Plugin(bundle) => apply_entries(bundle.string_entries_mut(), translations),
            Self::Pex(bundle) => apply_entries(bundle.string_entries_mut(), translations),
            Self::Scaleform(bundle) => apply_entries(bundle.string_entries_mut(), translations),
        }
    }

    async fn write(self, settings: &WorkspaceSettings) -> Result<Vec<FileAsset>, WorkspaceError> {
        match self {
            Self::Plugin(bundle) => {
                let output = write_localization(
                    bundle,
                    WriteOptions::new(settings.game_release, settings.asset_language),
                )
                .await?;
                Ok(output.into_files())
            }
            Self::Pex(bundle) => Ok(vec![write_pex_strings(bundle)?]),
            Self::Scaleform(bundle) => Ok(write_scaleform_translations(bundle)?.into_files()),
        }
    }
}

pub async fn export_translations(
    options: ExportTranslationsOptions,
) -> Result<ExportSummary, WorkspaceError> {
    let source_root = absolute_existing_path(&options.source_root)?;
    info!(
        source_root = %source_root,
        workspace = %options.workspace,
        force = options.force,
        "starting workspace export"
    );
    ensure_workspace_outside_source(&source_root, &options.workspace)?;
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    prepare_workspace_for_export(&options.workspace, options.force)?;
    let read = read_mod_root(&source_root, ReadModOptions::default())?;
    let bundles = read_editable_bundles(&read.files, &options.settings).await?;
    let mut records = Vec::new();
    for bundle in &bundles {
        bundle.collect_records(&options.settings, &mut records);
    }
    records
        .sort_by(|left, right| (&left.file, &left.record.id).cmp(&(&right.file, &right.record.id)));
    write_translation_package(
        &options.workspace,
        &source_root,
        &options.settings,
        &records,
    )?;
    info!(entries = records.len(), "finished workspace export");
    debug!(entries = records.len(), "exported translation package");
    Ok(ExportSummary {
        entries: records.len(),
    })
}

pub async fn import_translations(
    options: ImportTranslationsOptions,
) -> Result<ImportSummary, WorkspaceError> {
    info!(
        workspace = %options.workspace,
        output = %options.output,
        source_root_override = options.source_root.is_some(),
        "starting workspace import"
    );
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    if !options.force {
        ensure_workspace_complete_for_import(&options.workspace)?;
    }
    let (settings, stored_source_root, mut translations) =
        read_translation_package(&options.workspace)?;
    let source_root = options.source_root.unwrap_or(stored_source_root);
    let read = read_mod_root(&source_root, ReadModOptions::default())?;
    let mut bundles = read_editable_bundles(&read.files, &settings).await?;
    let mut applied_entries = 0;
    for bundle in &mut bundles {
        applied_entries += bundle.apply_translations(&mut translations);
    }
    if let Some(id) = translations.keys().next() {
        return Err(WorkspaceError::UnknownTranslationId { id: id.clone() });
    }

    let mut written_candidates = Vec::new();
    for bundle in bundles {
        written_candidates.extend(bundle.write(&settings).await?);
    }
    let changed = changed_assets(&read.files, written_candidates)?;
    ensure_output_outside_source(&source_root, &options.output)?;
    let written_files = write_output_assets(&options.output, &changed)?;
    debug!(
        applied_entries,
        written_files, "imported translation package"
    );
    info!(applied_entries, written_files, "finished workspace import");
    Ok(ImportSummary {
        applied_entries,
        written_files,
    })
}

fn ensure_workspace_complete_for_import(workspace: &Utf8Path) -> Result<(), WorkspaceError> {
    let count = count_batch(CountBatchOptions {
        workspace: workspace.to_owned(),
        file: None,
    })?;
    if count.claimable == 0 && count.claimed == 0 && count.diagnostics == 0 {
        return Ok(());
    }
    Err(WorkspaceError::WorkspaceIncomplete {
        claimable: count.claimable,
        claimed: count.claimed,
        diagnostics: count.diagnostics,
        empty: count.empty,
        memory_prefilled: count.memory_prefilled,
    })
}

fn prepare_workspace_for_export(workspace: &Utf8Path, force: bool) -> Result<(), WorkspaceError> {
    if force {
        remove_generated_workspace_artifacts(workspace)?;
        return Ok(());
    }
    if !workspace.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(workspace).map_err(|source| WorkspaceError::ReadFile {
        path: workspace.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceError::ReadFile {
            path: workspace.to_owned(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "workspace path is not valid UTF-8".to_string(),
            }
        })?;
        let Some(name) = path.file_name() else {
            continue;
        };
        match name {
            "lock" => {}
            "stringer.toml" => {}
            "knowledge" => validate_workspace_knowledge_inputs(&path)?,
            "workspace.json" | "entries" | "batches" => {
                return Err(WorkspaceError::InvalidTranslationPackagePath {
                    path: workspace.to_string(),
                    message: format!(
                        "workspace already contains generated artifact `{name}`; use --force to replace generated artifacts"
                    ),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_workspace_knowledge_inputs(path: &Utf8Path) -> Result<(), WorkspaceError> {
    if !path.is_dir() {
        return Err(WorkspaceError::InvalidTranslationPackagePath {
            path: path.to_string(),
            message: "workspace knowledge path must be a directory".to_string(),
        });
    }
    for entry in fs::read_dir(path).map_err(|source| WorkspaceError::ReadFile {
        path: path.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        let entry_path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "workspace knowledge path is not valid UTF-8".to_string(),
            }
        })?;
        let Some(name) = entry_path.file_name() else {
            continue;
        };
        if !matches!(name, "terms" | "memory" | "rules") {
            return Err(WorkspaceError::InvalidTranslationPackagePath {
                path: entry_path.to_string(),
                message: "workspace knowledge may only contain terms, memory, and rules inputs"
                    .to_string(),
            });
        }
        if !entry_path.is_dir() {
            return Err(WorkspaceError::InvalidTranslationPackagePath {
                path: entry_path.to_string(),
                message: "workspace knowledge input path must be a directory".to_string(),
            });
        }
    }
    Ok(())
}

fn remove_generated_workspace_artifacts(workspace: &Utf8Path) -> Result<(), WorkspaceError> {
    remove_file_if_exists(&workspace.join("workspace.json"))?;
    remove_dir_if_exists(&workspace.join("entries"))?;
    remove_dir_if_exists(&workspace.join("batches"))?;
    remove_file_if_exists(&workspace.join("knowledge/index.sqlite"))?;
    Ok(())
}

fn remove_file_if_exists(path: &Utf8Path) -> Result<(), WorkspaceError> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path).map_err(|source| WorkspaceError::WriteFile {
        path: path.to_owned(),
        source,
    })
}

fn remove_dir_if_exists(path: &Utf8Path) -> Result<(), WorkspaceError> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).map_err(|source| WorkspaceError::WriteFile {
        path: path.to_owned(),
        source,
    })
}

fn absolute_existing_path(path: &Utf8Path) -> Result<Utf8PathBuf, WorkspaceError> {
    let canonical = fs::canonicalize(path).map_err(|source| WorkspaceError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let path = Utf8PathBuf::from_path_buf(canonical).map_err(|path| {
        WorkspaceError::InvalidLogicalPath {
            path: path.display().to_string(),
            message: "source root path is not valid UTF-8".to_string(),
        }
    })?;
    Ok(strip_windows_verbatim_prefix(path))
}

#[cfg(windows)]
fn strip_windows_verbatim_prefix(path: Utf8PathBuf) -> Utf8PathBuf {
    let text = path.as_str();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return Utf8PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return Utf8PathBuf::from(rest.to_string());
    }
    path
}

#[cfg(not(windows))]
fn strip_windows_verbatim_prefix(path: Utf8PathBuf) -> Utf8PathBuf {
    path
}

async fn read_editable_bundles(
    files: &FileBundle,
    settings: &WorkspaceSettings,
) -> Result<Vec<EditableBundle>, WorkspaceError> {
    let mut bundles = Vec::new();
    let string_assets = files.strings().cloned().collect::<Vec<_>>();

    for plugin in files.plugins().cloned() {
        let mut plugin_files = Vec::with_capacity(string_assets.len() + 1);
        plugin_files.push(plugin);
        plugin_files.extend(string_assets.iter().cloned());
        let bundle = read_localization(
            FileBundle::new(plugin_files),
            ReadOptions::new(settings.game_release, settings.asset_language)
                .with_extraction_filters(settings.extraction_filters.clone()),
        )
        .await?;
        bundles.push(EditableBundle::Plugin(bundle));
    }

    for asset in files.pex().cloned() {
        bundles.push(EditableBundle::Pex(read_pex_strings(
            asset,
            ReadPexOptions::default().with_extraction_filters(settings.extraction_filters.clone()),
        )?));
    }

    let scaleform = read_scaleform_translations(files.clone(), settings.asset_language)?;
    if !scaleform.string_entries().is_empty() {
        bundles.push(EditableBundle::Scaleform(scaleform));
    }

    Ok(bundles)
}

fn apply_entries(
    entries: &mut [impl StringEntryView],
    translations: &mut BTreeMap<String, String>,
) -> usize {
    let mut applied = 0;
    for entry in entries {
        let id = external_entry_id(entry.string_entry());
        if let Some(text) = translations.remove(&id) {
            entry.string_entry_mut().set_text(text);
            applied += 1;
        }
    }
    applied
}
