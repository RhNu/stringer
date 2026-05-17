use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use stringer_pipeline::{PipelineAnnotation, PipelineDiagnostic};
use stringer_workspace_core::fsutil::{replace_file, temp_path};
use stringer_workspace_core::{
    BATCH_FORMAT_VERSION, BatchFile, TranslationMeta, TranslationRecord, WorkspaceLock,
    read_batch_file, read_translation_package_records, unix_ms, write_translation_package_records,
};

use crate::{WorkspaceOpsError, workspace_context_label};

const BATCH_WORK_DIR: &str = "batch-work";
pub(crate) const SKIP_REASONS: &[&str] = &[
    "not_translatable",
    "source_is_target",
    "identifier_or_token",
    "duplicate_or_obsolete",
    "needs_manual_review",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBatchOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchRead {
    pub batch_id: String,
    pub revision: u64,
    pub total_entries: usize,
    pub open_entries: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    pub entries: Vec<BatchReadEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchReadEntry {
    pub key: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_translation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub context_label: String,
    pub hint_count: usize,
    pub diagnostic_count: usize,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBatchDetailOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchDetail {
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<BatchDetailEntry>,
    pub missing_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BatchDetailEntry {
    pub key: String,
    pub file: String,
    pub id: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation_meta: Option<TranslationMeta>,
    pub context: BTreeMap<String, String>,
    pub hints: Vec<PipelineAnnotation>,
    pub diagnostics: Vec<PipelineDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchSubmitOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<BatchSubmitEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSubmitEntry {
    pub key: String,
    pub action: BatchSubmitAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchSubmitAction {
    Translate,
    Skip,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchSubmitSummary {
    pub batch_id: String,
    pub revision: u64,
    pub applied_entries: usize,
    pub ignored_entries: usize,
    pub rejected_entries: usize,
    pub remaining_entries: usize,
    pub next_read_offset: usize,
    pub results: Vec<BatchSubmitEntryResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchSubmitEntryResult {
    pub key: String,
    pub status: BatchSubmitStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchSubmitStatus {
    Applied,
    Ignored,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchExportOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
    pub out: Option<Utf8PathBuf>,
    pub format: BatchExportFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BatchExportSummary {
    pub path: String,
    pub format: BatchExportFormat,
    pub entries: usize,
}

#[derive(Debug, Clone)]
struct HydratedBatchEntry {
    key: String,
    file: String,
    record: TranslationRecord,
}

#[derive(Debug, Clone)]
struct ValidSubmit {
    key: String,
    id: String,
    action: BatchSubmitAction,
    translation: Option<String>,
    skip_reason: Option<String>,
}

pub fn read_batch(options: ReadBatchOptions) -> Result<BatchRead, WorkspaceOpsError> {
    let batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let entries = hydrate_batch_entries(&options.workspace, &batch)?;
    let total = entries.len();
    let start = options.offset.min(total);
    let end = start.saturating_add(options.limit).min(total);
    let page_entries = entries
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(compact_entry)
        .collect();
    let revision = batch.revision;
    Ok(BatchRead {
        batch_id: batch.batch_id,
        revision,
        total_entries: total,
        open_entries: total,
        offset: start,
        limit: options.limit,
        next_offset: (end < total).then_some(end),
        entries: page_entries,
    })
}

pub fn read_batch_detail(
    options: ReadBatchDetailOptions,
) -> Result<BatchDetail, WorkspaceOpsError> {
    if options.keys.is_empty() {
        return Err(WorkspaceOpsError::BatchDetailKeysRequired {
            batch_id: options.batch_id,
        });
    }
    let batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let entries = hydrate_batch_entries(&options.workspace, &batch)?;
    let requested_keys = options.keys;
    let requested = requested_keys.iter().cloned().collect::<BTreeSet<_>>();
    let details = entries
        .into_iter()
        .filter(|entry| requested.contains(&entry.key))
        .map(|entry| detail_entry(entry, &batch.batch_id))
        .collect::<Vec<_>>();
    let found = details
        .iter()
        .map(|entry| entry.key.clone())
        .collect::<BTreeSet<_>>();
    let mut seen_missing = BTreeSet::new();
    let missing_keys = requested_keys
        .into_iter()
        .filter(|key| !found.contains(key) && seen_missing.insert(key.clone()))
        .collect();
    let revision = batch.revision;
    Ok(BatchDetail {
        batch_id: batch.batch_id,
        revision,
        entries: details,
        missing_keys,
    })
}

pub fn submit_batch(options: BatchSubmitOptions) -> Result<BatchSubmitSummary, WorkspaceOpsError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let current_revision = batch.revision;
    if current_revision != options.revision {
        return Err(WorkspaceOpsError::BatchRevisionConflict {
            batch_id: options.batch_id,
            expected: options.revision,
            current: current_revision,
        });
    }

    let original_entries = batch.entries.clone();
    let key_to_id = original_entries
        .iter()
        .map(|entry| (entry.key.clone(), entry.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut results = Vec::new();
    let mut valid = Vec::new();
    let mut ignored = 0;
    let mut rejected = 0;
    for entry in options.entries {
        if !seen.insert(entry.key.clone()) {
            rejected += 1;
            results.push(rejected_result(
                entry.key,
                "duplicate key in submit entries",
            ));
            continue;
        }
        let Some(id) = key_to_id.get(&entry.key).cloned() else {
            rejected += 1;
            results.push(rejected_result(
                entry.key,
                "key is not in remaining batch entries",
            ));
            continue;
        };
        match validate_submit_entry(entry, id) {
            SubmitValidation::Valid(valid_entry) => {
                results.push(BatchSubmitEntryResult {
                    key: valid_entry.key.clone(),
                    status: BatchSubmitStatus::Applied,
                    message: None,
                });
                valid.push(valid_entry);
            }
            SubmitValidation::Ignored(key) => {
                ignored += 1;
                results.push(BatchSubmitEntryResult {
                    key,
                    status: BatchSubmitStatus::Ignored,
                    message: None,
                });
            }
            SubmitValidation::Rejected { key, message } => {
                rejected += 1;
                results.push(rejected_result(key, message));
            }
        }
    }

    if valid.is_empty() {
        return Ok(BatchSubmitSummary {
            batch_id: batch.batch_id,
            revision: current_revision,
            applied_entries: 0,
            ignored_entries: ignored,
            rejected_entries: rejected,
            remaining_entries: original_entries.len(),
            next_read_offset: 0,
            results,
        });
    }

    let mut package = read_translation_package_records(&options.workspace)?;
    let mut records = BTreeMap::<String, &mut TranslationRecord>::new();
    for file in &mut package.files {
        for record in &mut file.records {
            records.insert(record.id.clone(), record);
        }
    }
    let mut applied_keys = BTreeSet::new();
    for entry in &valid {
        let Some(record) = records.get_mut(&entry.id) else {
            rejected += 1;
            mark_result_rejected(&mut results, &entry.key, "translation id does not exist");
            continue;
        };
        match entry.action {
            BatchSubmitAction::Translate => {
                record.translation = entry.translation.clone();
                record.translation_meta = Some(TranslationMeta {
                    origin: Some("agent".to_string()),
                    updated_at_unix_ms: Some(unix_ms()),
                    skip_reason: None,
                });
            }
            BatchSubmitAction::Skip => {
                record.translation = None;
                record.translation_meta = Some(TranslationMeta {
                    origin: Some("skipped".to_string()),
                    updated_at_unix_ms: Some(unix_ms()),
                    skip_reason: entry.skip_reason.clone(),
                });
            }
            BatchSubmitAction::Pending => {}
        }
        applied_keys.insert(entry.key.clone());
    }

    let applied = applied_keys.len();
    let revision = if applied > 0 {
        current_revision.saturating_add(1)
    } else {
        current_revision
    };
    let remaining_entries = original_entries
        .into_iter()
        .filter(|entry| !applied_keys.contains(&entry.key))
        .collect::<Vec<_>>();
    write_translation_package_records(&options.workspace, &package)?;
    batch.batch_format_version = BATCH_FORMAT_VERSION;
    batch.revision = revision;
    batch.entries = remaining_entries;
    if batch.entries.is_empty() {
        remove_batch_file(&options.workspace, &batch.batch_id)?;
    } else {
        write_batch_file(&options.workspace, &batch)?;
    }

    Ok(BatchSubmitSummary {
        batch_id: batch.batch_id,
        revision,
        applied_entries: applied,
        ignored_entries: ignored,
        rejected_entries: rejected,
        remaining_entries: batch.entries.len(),
        next_read_offset: 0,
        results,
    })
}

pub fn export_batch_submission(
    options: BatchExportOptions,
) -> Result<BatchExportSummary, WorkspaceOpsError> {
    let batch = read_batch_file(&options.workspace, &options.batch_id)?;
    let entries = hydrate_batch_entries(&options.workspace, &batch)?;
    let extension = match options.format {
        BatchExportFormat::Json => "json",
        BatchExportFormat::Csv => "csv",
    };
    let path = options.out.unwrap_or_else(|| {
        options
            .workspace
            .join(BATCH_WORK_DIR)
            .join(&options.batch_id)
            .join(format!("patch.{extension}"))
    });
    match options.format {
        BatchExportFormat::Json => write_json_submission(&path, &batch, &entries)?,
        BatchExportFormat::Csv => write_csv_submission(&path, &batch, &entries)?,
    }
    Ok(BatchExportSummary {
        path: path.as_str().replace('\\', "/"),
        format: options.format,
        entries: entries.len(),
    })
}

enum SubmitValidation {
    Valid(ValidSubmit),
    Ignored(String),
    Rejected { key: String, message: &'static str },
}

fn validate_submit_entry(entry: BatchSubmitEntry, id: String) -> SubmitValidation {
    match entry.action {
        BatchSubmitAction::Pending => {
            if entry.translation.is_some() || entry.skip_reason.is_some() {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "pending action must not include translation or skip_reason",
                };
            }
            SubmitValidation::Ignored(entry.key)
        }
        BatchSubmitAction::Translate => {
            if entry.skip_reason.is_some() {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "translate action must not include skip_reason",
                };
            }
            if entry
                .translation
                .as_deref()
                .is_none_or(|translation| translation.trim().is_empty())
            {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "translate action requires non-empty translation",
                };
            }
            SubmitValidation::Valid(ValidSubmit {
                key: entry.key,
                id,
                action: entry.action,
                translation: entry.translation,
                skip_reason: None,
            })
        }
        BatchSubmitAction::Skip => {
            if entry.translation.is_some() {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "skip action must not include translation",
                };
            }
            let Some(skip_reason) = entry.skip_reason else {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "skip action requires skip_reason",
                };
            };
            if !SKIP_REASONS.contains(&skip_reason.as_str()) {
                return SubmitValidation::Rejected {
                    key: entry.key,
                    message: "unsupported skip_reason",
                };
            }
            SubmitValidation::Valid(ValidSubmit {
                key: entry.key,
                id,
                action: entry.action,
                translation: None,
                skip_reason: Some(skip_reason),
            })
        }
    }
}

fn compact_entry(entry: &HydratedBatchEntry) -> BatchReadEntry {
    BatchReadEntry {
        key: entry.key.clone(),
        source: entry.record.source.clone(),
        current_translation: entry.record.translation.clone(),
        origin: entry
            .record
            .translation_meta
            .as_ref()
            .and_then(|meta| meta.origin.clone()),
        context_label: workspace_context_label(&entry.file, &entry.record.context),
        hint_count: entry.record.hints.len(),
        diagnostic_count: entry.record.diagnostics.len(),
        diagnostic_codes: entry
            .record
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_string())
            .collect(),
    }
}

fn detail_entry(entry: HydratedBatchEntry, batch_id: &str) -> BatchDetailEntry {
    BatchDetailEntry {
        key: entry.key,
        file: entry.file,
        id: entry.record.id,
        source: entry.record.source,
        translation: entry.record.translation,
        translation_meta: entry.record.translation_meta,
        context: entry.record.context,
        hints: entry.record.hints,
        diagnostics: entry.record.diagnostics,
        claimed_by: Some(batch_id.to_string()),
    }
}

fn hydrate_batch_entries(
    workspace: &Utf8Path,
    batch: &BatchFile,
) -> Result<Vec<HydratedBatchEntry>, WorkspaceOpsError> {
    let package = read_translation_package_records(workspace)?;
    let mut records = BTreeMap::new();
    for file in package.files {
        for record in file.records {
            records.insert(record.id.clone(), (file.manifest_file.path.clone(), record));
        }
    }
    let mut hydrated = Vec::new();
    for entry in &batch.entries {
        let Some((file, record)) = records.get(&entry.id) else {
            return Err(WorkspaceOpsError::UnknownTranslationId {
                id: entry.id.clone(),
            });
        };
        hydrated.push(HydratedBatchEntry {
            key: entry.key.clone(),
            file: file.clone(),
            record: record.clone(),
        });
    }
    Ok(hydrated)
}

fn rejected_result(key: String, message: impl Into<String>) -> BatchSubmitEntryResult {
    BatchSubmitEntryResult {
        key,
        status: BatchSubmitStatus::Rejected,
        message: Some(message.into()),
    }
}

fn mark_result_rejected(results: &mut [BatchSubmitEntryResult], key: &str, message: &'static str) {
    if let Some(result) = results.iter_mut().find(|result| result.key == key) {
        result.status = BatchSubmitStatus::Rejected;
        result.message = Some(message.to_string());
    }
}

fn write_json_submission(
    path: &Utf8Path,
    batch: &BatchFile,
    entries: &[HydratedBatchEntry],
) -> Result<(), WorkspaceOpsError> {
    let submission = serde_json::json!({
        "batch_id": &batch.batch_id,
        "revision": batch.revision,
        "entries": entries.iter().map(export_entry).collect::<Vec<_>>(),
    });
    write_json_atomic(path, &submission)
}

fn export_entry(entry: &HydratedBatchEntry) -> serde_json::Value {
    let compact = compact_entry(entry);
    serde_json::json!({
        "key": compact.key,
        "source": compact.source,
        "current_translation": compact.current_translation,
        "context_label": compact.context_label,
        "diagnostic_codes": compact.diagnostic_codes,
        "action": "pending",
        "translation": null,
        "skip_reason": null,
    })
}

fn write_csv_submission(
    path: &Utf8Path,
    batch: &BatchFile,
    entries: &[HydratedBatchEntry],
) -> Result<(), WorkspaceOpsError> {
    let mut text = format!(
        "# stringer batch_id={} revision={}\n",
        batch.batch_id, batch.revision
    );
    text.push_str(
        "key,source,current_translation,context_label,diagnostic_codes,action,translation,skip_reason\n",
    );
    for entry in entries {
        let compact = compact_entry(entry);
        let columns = [
            compact.key,
            compact.source,
            compact.current_translation.unwrap_or_default(),
            compact.context_label,
            compact.diagnostic_codes.join("|"),
            "pending".to_string(),
            String::new(),
            String::new(),
        ];
        text.push_str(
            &columns
                .iter()
                .map(|column| csv_escape(column))
                .collect::<Vec<_>>()
                .join(","),
        );
        text.push('\n');
    }
    write_text_atomic(path, &text)
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn remove_batch_file(workspace: &Utf8Path, batch_id: &str) -> Result<(), WorkspaceOpsError> {
    let path = workspace.join("batches").join(format!("{batch_id}.json"));
    fs::remove_file(&path).map_err(|source| {
        stringer_workspace_core::WorkspaceCoreError::WriteFile { path, source }
    })?;
    Ok(())
}

fn write_batch_file(workspace: &Utf8Path, batch: &BatchFile) -> Result<(), WorkspaceOpsError> {
    write_json_atomic(
        &workspace
            .join("batches")
            .join(format!("{}.json", batch.batch_id)),
        batch,
    )
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

fn write_text_atomic(path: &Utf8Path, value: &str) -> Result<(), WorkspaceOpsError> {
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
    fs::write(&temp, value).map_err(|source| {
        stringer_workspace_core::WorkspaceCoreError::WriteFile {
            path: temp.clone(),
            source,
        }
    })?;
    replace_file(&temp, path)?;
    Ok(())
}
