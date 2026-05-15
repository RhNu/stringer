use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::WorkspaceError;
use crate::fsutil::{replace_file, temp_path};
use crate::lock::{WorkspaceLock, unix_ms};
use crate::package::{
    TranslationMeta, TranslationRecord, read_translation_package_records,
    write_translation_package_records,
};

const BATCHES_DIR: &str = "batches";

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
    pub entries: Vec<ClaimedBatchEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClaimedBatchEntry {
    pub id: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_meta: Option<TranslationMeta>,
    pub context: BTreeMap<String, String>,
    pub hints: Vec<stringer_pipeline::PipelineAnnotation>,
    pub diagnostics: Vec<stringer_pipeline::PipelineDiagnostic>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BatchFile {
    schema_version: u32,
    batch_id: String,
    created_at_unix_ms: u128,
    scope: BatchScope,
    entry_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct BatchScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

pub fn count_batch(options: CountBatchOptions) -> Result<BatchCount, WorkspaceError> {
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

pub fn claim_batch(options: ClaimBatchOptions) -> Result<ClaimedBatch, WorkspaceError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let package = read_translation_package_records(&options.workspace)?;
    let file = normalize_file_filter(options.file.as_deref());
    validate_file_filter(&package, file.as_deref())?;
    let claimed = claimed_entry_ids(&options.workspace)?;
    let mut entries = Vec::new();
    let mut ids = Vec::new();
    for file_records in package.files.iter().filter(|file_records| {
        file.as_deref()
            .is_none_or(|expected| file_records.manifest_file.path == expected)
    }) {
        for record in &file_records.records {
            if entries.len() >= options.limit {
                break;
            }
            if claimed.contains(&record.id) || !is_claim_eligible(record) {
                continue;
            }
            ids.push(record.id.clone());
            entries.push(claimed_entry(record));
        }
        if entries.len() >= options.limit {
            break;
        }
    }
    if entries.is_empty() {
        return Ok(ClaimedBatch {
            batch_id: None,
            entries,
        });
    }

    let batch_id = format!("b{}-{}", unix_ms(), std::process::id());
    let batch = BatchFile {
        schema_version: crate::package::SCHEMA_VERSION,
        batch_id: batch_id.clone(),
        created_at_unix_ms: unix_ms(),
        scope: BatchScope { file },
        entry_ids: ids,
    };
    write_batch_file(&options.workspace, &batch)?;
    Ok(ClaimedBatch {
        batch_id: Some(batch_id),
        entries,
    })
}

pub fn apply_batch_patch(
    options: ApplyBatchPatchOptions,
) -> Result<ApplyBatchPatchSummary, WorkspaceError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let mut seen = BTreeSet::new();
    for entry in &options.entries {
        if !seen.insert(entry.id.clone()) {
            return Err(WorkspaceError::DuplicateBatchPatchId {
                id: entry.id.clone(),
            });
        }
        if entry.translation.is_none() {
            return Err(WorkspaceError::MissingBatchPatchTranslation {
                id: entry.id.clone(),
            });
        }
        if !batch.entry_ids.contains(&entry.id) {
            return Err(WorkspaceError::BatchEntryNotClaimed {
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
            return Err(WorkspaceError::UnknownTranslationId {
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

pub fn release_batch(options: ReleaseBatchOptions) -> Result<ReleaseBatchSummary, WorkspaceError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let released_entries = batch.entry_ids.len();
    remove_batch_file(&options.workspace, &options.batch_id)?;
    Ok(ReleaseBatchSummary { released_entries })
}

pub(crate) fn claimed_entry_ids(workspace: &Utf8Path) -> Result<BTreeSet<String>, WorkspaceError> {
    let mut ids = BTreeSet::new();
    for batch in read_batch_files(workspace)? {
        ids.extend(batch.entry_ids);
    }
    Ok(ids)
}

pub(crate) fn claimed_entry_batches(
    workspace: &Utf8Path,
) -> Result<BTreeMap<String, String>, WorkspaceError> {
    let mut claims = BTreeMap::new();
    for batch in read_batch_files(workspace)? {
        for id in batch.entry_ids {
            claims.insert(id, batch.batch_id.clone());
        }
    }
    Ok(claims)
}

pub(crate) fn batch_entry_ids(
    workspace: &Utf8Path,
    batch_id: &str,
) -> Result<Vec<String>, WorkspaceError> {
    Ok(read_batch_file(workspace, batch_id)?.entry_ids)
}

fn validate_file_filter(
    package: &crate::package::TranslationPackageRecords,
    file: Option<&str>,
) -> Result<(), WorkspaceError> {
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
    Err(WorkspaceError::InvalidTranslationPackagePath {
        path: file.to_string(),
        message: "entry file is not listed in workspace.json".to_string(),
    })
}

fn is_claim_eligible(record: &TranslationRecord) -> bool {
    if matches!(translation_origin(record), Some("agent" | "manual")) {
        return false;
    }
    is_empty_translation(record) || translation_origin(record) == Some("memory")
}

fn claimed_entry(record: &TranslationRecord) -> ClaimedBatchEntry {
    ClaimedBatchEntry {
        id: record.id.clone(),
        source: record.source.clone(),
        translation: record.translation.clone(),
        translation_meta: record.translation_meta.clone(),
        context: record.context.clone(),
        hints: record.hints.clone(),
        diagnostics: record.diagnostics.clone(),
    }
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

fn read_batch_files(workspace: &Utf8Path) -> Result<Vec<BatchFile>, WorkspaceError> {
    let root = workspace.join(BATCHES_DIR);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut batches = Vec::new();
    for entry in fs::read_dir(&root).map_err(|source| WorkspaceError::ReadFile {
        path: root.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceError::ReadFile {
            path: root.clone(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "batch path is not valid UTF-8".to_string(),
            }
        })?;
        if path.extension() == Some("json") {
            batches.push(read_json(&path)?);
        }
    }
    Ok(batches)
}

fn read_batch_file(workspace: &Utf8Path, batch_id: &str) -> Result<BatchFile, WorkspaceError> {
    validate_batch_id(batch_id)?;
    let path = batch_path(workspace, batch_id);
    if !path.exists() {
        return Err(WorkspaceError::BatchNotFound {
            batch_id: batch_id.to_string(),
        });
    }
    read_json(&path)
}

fn write_batch_file(workspace: &Utf8Path, batch: &BatchFile) -> Result<(), WorkspaceError> {
    let path = batch_path(workspace, &batch.batch_id);
    write_json_atomic(&path, batch)
}

fn remove_batch_file(workspace: &Utf8Path, batch_id: &str) -> Result<(), WorkspaceError> {
    validate_batch_id(batch_id)?;
    let path = batch_path(workspace, batch_id);
    fs::remove_file(&path).map_err(|source| WorkspaceError::WriteFile { path, source })
}

fn batch_path(workspace: &Utf8Path, batch_id: &str) -> Utf8PathBuf {
    workspace.join(BATCHES_DIR).join(format!("{batch_id}.json"))
}

fn validate_batch_id(batch_id: &str) -> Result<(), WorkspaceError> {
    if !batch_id.is_empty() && !batch_id.contains(['/', '\\']) {
        return Ok(());
    }
    Err(WorkspaceError::InvalidTranslationPackagePath {
        path: batch_id.to_string(),
        message: "batch id must be a file name, not a path".to_string(),
    })
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Utf8Path) -> Result<T, WorkspaceError> {
    let text = fs::read_to_string(path).map_err(|source| WorkspaceError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| WorkspaceError::Json {
        path: path.to_owned(),
        source,
    })
}

fn write_json_atomic<T: Serialize>(path: &Utf8Path, value: &T) -> Result<(), WorkspaceError> {
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let temp = temp_path(path, unix_ms().to_string());
    {
        let mut file = fs::File::create(&temp).map_err(|source| WorkspaceError::WriteFile {
            path: temp.clone(),
            source,
        })?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|source| WorkspaceError::Json {
            path: temp.clone(),
            source,
        })?;
        file.write_all(b"\n")
            .map_err(|source| WorkspaceError::WriteFile {
                path: temp.clone(),
                source,
            })?;
        file.flush().map_err(|source| WorkspaceError::WriteFile {
            path: temp.clone(),
            source,
        })?;
    }
    replace_file(&temp, path)
}

fn normalize_file_filter(file: Option<&str>) -> Option<String> {
    file.map(|value| value.replace('\\', "/"))
}
