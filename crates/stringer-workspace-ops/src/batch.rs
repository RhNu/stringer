use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use stringer_workspace_core::fsutil::{replace_file, temp_path};
use stringer_workspace_core::{
    BatchFile, BatchScope, TranslationMeta, TranslationRecord, WorkspaceLock, claimed_entry_ids,
    read_batch_file, read_translation_package_records, unix_ms, validate_batch_id,
    write_translation_package_records,
};

use crate::WorkspaceOpsError;

const BATCHES_DIR: &str = "batches";
static BATCH_ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountBatchOptions {
    pub workspace: Utf8PathBuf,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BatchCount {
    pub total: usize,
    pub empty: usize,
    pub memory_prefilled: usize,
    pub translated: usize,
    pub claimed: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimBatchOptions {
    pub workspace: Utf8PathBuf,
    pub file: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClaimedBatch {
    pub batch_id: Option<String>,
    pub claimed_entries: usize,
    pub scope: BatchScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApplyBatchPatchInput {
    pub batch_id: String,
    pub entries: Vec<ApplyBatchPatchEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ApplyBatchPatchEntry {
    pub id: String,
    pub translation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyBatchPatchOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
    pub entries: Vec<ApplyBatchPatchEntry>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ApplyBatchPatchSummary {
    pub applied_entries: usize,
    pub remaining_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBatchOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ReleaseBatchSummary {
    pub released_entries: usize,
}

pub fn count_batch(options: CountBatchOptions) -> Result<BatchCount, WorkspaceOpsError> {
    let package = read_translation_package_records(&options.workspace)?;
    let file = normalize_file_filter(options.file.as_deref());
    validate_file_filter(&package, file.as_deref())?;
    let claimed = claimed_entry_ids(&options.workspace)?;
    let mut count = BatchCount::default();
    for file_records in package.files.iter().filter(|file_records| {
        file.as_deref()
            .is_none_or(|expected| file_records.manifest_file.path == expected)
    }) {
        for record in &file_records.records {
            count.total += 1;
            if is_empty_translation(record) {
                count.empty += 1;
            } else if translation_origin(record) == Some("memory") {
                count.memory_prefilled += 1;
            } else {
                count.translated += 1;
            }
            if claimed.contains(&record.id) {
                count.claimed += 1;
            }
            if !record.diagnostics.is_empty() {
                count.diagnostics += 1;
            }
        }
    }
    Ok(count)
}

pub fn claim_batch(options: ClaimBatchOptions) -> Result<ClaimedBatch, WorkspaceOpsError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let package = read_translation_package_records(&options.workspace)?;
    let file = normalize_file_filter(options.file.as_deref());
    validate_file_filter(&package, file.as_deref())?;
    let claimed = claimed_entry_ids(&options.workspace)?;
    let scope = BatchScope { file: file.clone() };
    let mut ids = Vec::new();
    for file_records in package.files.iter().filter(|file_records| {
        file.as_deref()
            .is_none_or(|expected| file_records.manifest_file.path == expected)
    }) {
        for record in &file_records.records {
            if ids.len() >= options.limit {
                break;
            }
            if claimed.contains(&record.id) || !is_claim_eligible(record) {
                continue;
            }
            ids.push(record.id.clone());
        }
        if ids.len() >= options.limit {
            break;
        }
    }
    if ids.is_empty() {
        return Ok(ClaimedBatch {
            batch_id: None,
            claimed_entries: 0,
            scope,
        });
    }

    let batch_id = next_batch_id(&options.workspace);
    let claimed_entries = ids.len();
    let batch = BatchFile {
        schema_version: stringer_workspace_core::SCHEMA_VERSION,
        batch_id: batch_id.clone(),
        created_at_unix_ms: unix_ms(),
        scope: scope.clone(),
        entry_ids: ids,
    };
    write_batch_file(&options.workspace, &batch)?;
    Ok(ClaimedBatch {
        batch_id: Some(batch_id),
        claimed_entries,
        scope,
    })
}

pub fn apply_batch_patch(
    options: ApplyBatchPatchOptions,
) -> Result<ApplyBatchPatchSummary, WorkspaceOpsError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let mut seen = BTreeSet::new();
    for entry in &options.entries {
        if !seen.insert(entry.id.clone()) {
            return Err(WorkspaceOpsError::DuplicateBatchPatchId {
                id: entry.id.clone(),
            });
        }
        if entry.translation.is_none() {
            return Err(WorkspaceOpsError::MissingBatchPatchTranslation {
                id: entry.id.clone(),
            });
        }
        if !batch.entry_ids.contains(&entry.id) {
            return Err(WorkspaceOpsError::BatchEntryNotClaimed {
                batch_id: options.batch_id.clone(),
                id: entry.id.clone(),
            });
        }
    }

    let mut package = read_translation_package_records(&options.workspace)?;
    let mut records = BTreeMap::<String, &mut TranslationRecord>::new();
    for file in &mut package.files {
        for record in &mut file.records {
            records.insert(record.id.clone(), record);
        }
    }
    for entry in &options.entries {
        let Some(record) = records.get_mut(&entry.id) else {
            return Err(WorkspaceOpsError::UnknownTranslationId {
                id: entry.id.clone(),
            });
        };
        record.translation = entry.translation.clone();
        record.translation_meta = Some(TranslationMeta {
            origin: Some("agent".to_string()),
            updated_at_unix_ms: Some(unix_ms()),
        });
    }
    let applied = options.entries.len();
    let applied_ids = options
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    batch
        .entry_ids
        .retain(|id| !applied_ids.contains(id.as_str()));
    let remaining = batch.entry_ids.len();

    write_translation_package_records(&options.workspace, &package)?;
    if batch.entry_ids.is_empty() {
        remove_batch_file(&options.workspace, &options.batch_id)?;
    } else {
        write_batch_file(&options.workspace, &batch)?;
    }
    Ok(ApplyBatchPatchSummary {
        applied_entries: applied,
        remaining_entries: remaining,
    })
}

pub fn release_batch(
    options: ReleaseBatchOptions,
) -> Result<ReleaseBatchSummary, WorkspaceOpsError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let released_entries = batch.entry_ids.len();
    remove_batch_file(&options.workspace, &options.batch_id)?;
    Ok(ReleaseBatchSummary { released_entries })
}

fn validate_file_filter(
    package: &stringer_workspace_core::TranslationPackageRecords,
    file: Option<&str>,
) -> Result<(), WorkspaceOpsError> {
    let Some(file) = file else {
        return Ok(());
    };
    if package
        .files
        .iter()
        .any(|candidate| candidate.manifest_file.path == file)
    {
        return Ok(());
    }
    Err(
        stringer_workspace_core::WorkspaceCoreError::InvalidTranslationPackagePath {
            path: file.to_string(),
            message: "entry file is not listed in workspace.json".to_string(),
        }
        .into(),
    )
}

fn is_claim_eligible(record: &TranslationRecord) -> bool {
    if matches!(translation_origin(record), Some("agent" | "manual")) {
        return false;
    }
    is_empty_translation(record) || translation_origin(record) == Some("memory")
}

fn is_empty_translation(record: &TranslationRecord) -> bool {
    record
        .translation
        .as_deref()
        .is_none_or(|translation| translation.is_empty())
}

fn translation_origin(record: &TranslationRecord) -> Option<&str> {
    record
        .translation_meta
        .as_ref()
        .and_then(|meta| meta.origin.as_deref())
}

fn write_batch_file(workspace: &Utf8Path, batch: &BatchFile) -> Result<(), WorkspaceOpsError> {
    let path = batch_path(workspace, &batch.batch_id);
    write_json_atomic(&path, batch)
}

fn remove_batch_file(workspace: &Utf8Path, batch_id: &str) -> Result<(), WorkspaceOpsError> {
    validate_batch_id(batch_id)?;
    let path = batch_path(workspace, batch_id);
    fs::remove_file(&path).map_err(|source| {
        stringer_workspace_core::WorkspaceCoreError::WriteFile { path, source }
    })?;
    Ok(())
}

fn batch_path(workspace: &Utf8Path, batch_id: &str) -> Utf8PathBuf {
    workspace.join(BATCHES_DIR).join(format!("{batch_id}.json"))
}

fn next_batch_id(workspace: &Utf8Path) -> String {
    loop {
        let sequence = BATCH_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let batch_id = format!("b{}-{}-{}", unix_ms(), std::process::id(), sequence);
        if !batch_path(workspace, &batch_id).exists() {
            return batch_id;
        }
    }
}

fn write_json_atomic<T: Serialize>(path: &Utf8Path, value: &T) -> Result<(), WorkspaceOpsError> {
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::WriteFile {
                path: parent.to_owned(),
                source,
            }
        })?;
    }
    let temp = temp_path(path, unix_ms().to_string());
    {
        let mut file = fs::File::create(&temp).map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::WriteFile {
                path: temp.clone(),
                source,
            }
        })?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::Json {
                path: temp.clone(),
                source,
            }
        })?;
        file.write_all(b"\n").map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::WriteFile {
                path: temp.clone(),
                source,
            }
        })?;
        file.flush().map_err(
            |source| stringer_workspace_core::WorkspaceCoreError::WriteFile {
                path: temp.clone(),
                source,
            },
        )?;
    }
    replace_file(&temp, path)?;
    Ok(())
}

fn normalize_file_filter(file: Option<&str>) -> Option<String> {
    file.map(|value| value.replace('\\', "/"))
}
