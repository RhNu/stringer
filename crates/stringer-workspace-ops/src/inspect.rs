use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use serde::Serialize;
use stringer_pipeline::{PipelineAnnotation, PipelineDiagnostic, PipelineDiagnosticSeverity};
use stringer_workspace_core::{
    TranslationManifestFile, TranslationMeta, TranslationRecord, batch_entry_ids,
    claimed_entry_batches, read_translation_manifest_files, read_translation_package_records,
    visit_translation_package_records_filtered,
};

use crate::WorkspaceOpsError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWorkspaceFilesOptions {
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkspaceInspectFiles {
    pub files: Vec<TranslationManifestFile>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InspectEntryStatus {
    #[default]
    All,
    Empty,
    Memory,
    Translated,
    Claimed,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWorkspaceEntriesOptions {
    pub workspace: Utf8PathBuf,
    pub file: Option<String>,
    pub status: InspectEntryStatus,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceInspectEntries {
    pub total: usize,
    pub entries: Vec<WorkspaceInspectEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceInspectEntry {
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
pub struct InspectWorkspaceEntryOptions {
    pub workspace: Utf8PathBuf,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWorkspaceBatchOptions {
    pub workspace: Utf8PathBuf,
    pub batch_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceInspectBatch {
    pub batch_id: String,
    pub entries: Vec<WorkspaceInspectEntry>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InspectDiagnosticSeverity {
    #[default]
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectWorkspaceDiagnosticsOptions {
    pub workspace: Utf8PathBuf,
    pub file: Option<String>,
    pub severity: InspectDiagnosticSeverity,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceInspectDiagnostics {
    pub total: usize,
    pub diagnostics: Vec<WorkspaceInspectDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkspaceInspectDiagnostic {
    pub entry_id: String,
    pub file: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    pub context: BTreeMap<String, String>,
    pub diagnostic: PipelineDiagnostic,
}

pub fn inspect_workspace_files(
    options: InspectWorkspaceFilesOptions,
) -> Result<WorkspaceInspectFiles, WorkspaceOpsError> {
    let files = read_translation_manifest_files(&options.workspace)?;
    Ok(WorkspaceInspectFiles { files })
}

pub fn inspect_workspace_entries(
    options: InspectWorkspaceEntriesOptions,
) -> Result<WorkspaceInspectEntries, WorkspaceOpsError> {
    let file = normalize_file_filter(options.file.as_deref());
    let claims = claimed_entry_batches(&options.workspace)?;
    let mut entries = Vec::new();
    let mut total = 0;
    visit_translation_package_records_filtered(
        &options.workspace,
        file.as_deref(),
        |manifest_file, record| {
            let claimed_by = claims.get(&record.id).cloned();
            if !entry_matches_status(&record, claimed_by.as_deref(), options.status) {
                return Ok(());
            }
            let index = total;
            total += 1;
            if is_in_page(index, options.limit, options.offset) {
                entries.push(inspect_entry(&manifest_file.path, &record, claimed_by));
            }
            Ok(())
        },
    )?;
    Ok(WorkspaceInspectEntries { total, entries })
}

pub fn inspect_workspace_entry(
    options: InspectWorkspaceEntryOptions,
) -> Result<WorkspaceInspectEntry, WorkspaceOpsError> {
    let package = read_translation_package_records(&options.workspace)?;
    let claims = claimed_entry_batches(&options.workspace)?;
    for file in &package.files {
        if let Some(record) = file.records.iter().find(|record| record.id == options.id) {
            return Ok(inspect_entry(
                &file.manifest_file.path,
                record,
                claims.get(&record.id).cloned(),
            ));
        }
    }
    Err(WorkspaceOpsError::UnknownTranslationId { id: options.id })
}

pub fn inspect_workspace_batch(
    options: InspectWorkspaceBatchOptions,
) -> Result<WorkspaceInspectBatch, WorkspaceOpsError> {
    let ids = batch_entry_ids(&options.workspace, &options.batch_id)?;
    let package = read_translation_package_records(&options.workspace)?;
    let mut records = BTreeMap::new();
    for file in &package.files {
        for record in &file.records {
            records.insert(record.id.as_str(), (&file.manifest_file.path, record));
        }
    }
    let mut entries = Vec::new();
    for id in ids {
        let Some((file, record)) = records.get(id.as_str()) else {
            return Err(WorkspaceOpsError::UnknownTranslationId { id });
        };
        entries.push(inspect_entry(file, record, Some(options.batch_id.clone())));
    }
    Ok(WorkspaceInspectBatch {
        batch_id: options.batch_id,
        entries,
    })
}

pub fn inspect_workspace_diagnostics(
    options: InspectWorkspaceDiagnosticsOptions,
) -> Result<WorkspaceInspectDiagnostics, WorkspaceOpsError> {
    let file = normalize_file_filter(options.file.as_deref());
    let mut diagnostics = Vec::new();
    let mut total = 0;
    visit_translation_package_records_filtered(
        &options.workspace,
        file.as_deref(),
        |manifest_file, record| {
            for diagnostic in &record.diagnostics {
                if !diagnostic_matches_severity(diagnostic, options.severity) {
                    continue;
                }
                let index = total;
                total += 1;
                if is_in_page(index, options.limit, options.offset) {
                    diagnostics.push(WorkspaceInspectDiagnostic {
                        entry_id: record.id.clone(),
                        file: manifest_file.path.clone(),
                        source: record.source.clone(),
                        translation: record.translation.clone(),
                        context: record.context.clone(),
                        diagnostic: diagnostic.clone(),
                    });
                }
            }
            Ok(())
        },
    )?;
    Ok(WorkspaceInspectDiagnostics { total, diagnostics })
}

fn inspect_entry(
    file: &str,
    record: &TranslationRecord,
    claimed_by: Option<String>,
) -> WorkspaceInspectEntry {
    WorkspaceInspectEntry {
        file: file.to_string(),
        id: record.id.clone(),
        source: record.source.clone(),
        translation: record.translation.clone(),
        translation_meta: record.translation_meta.clone(),
        context: record.context.clone(),
        hints: record.hints.clone(),
        diagnostics: record.diagnostics.clone(),
        claimed_by,
    }
}

fn entry_matches_status(
    record: &TranslationRecord,
    claimed_by: Option<&str>,
    status: InspectEntryStatus,
) -> bool {
    match status {
        InspectEntryStatus::All => true,
        InspectEntryStatus::Empty => is_empty_translation(record),
        InspectEntryStatus::Memory => {
            !is_empty_translation(record) && translation_origin(record) == Some("memory")
        }
        InspectEntryStatus::Translated => {
            !is_empty_translation(record) && translation_origin(record) != Some("memory")
        }
        InspectEntryStatus::Claimed => claimed_by.is_some(),
        InspectEntryStatus::Diagnostic => !record.diagnostics.is_empty(),
    }
}

fn diagnostic_matches_severity(
    diagnostic: &PipelineDiagnostic,
    severity: InspectDiagnosticSeverity,
) -> bool {
    match severity {
        InspectDiagnosticSeverity::All => true,
        InspectDiagnosticSeverity::Error => {
            diagnostic.severity() == PipelineDiagnosticSeverity::Error
        }
        InspectDiagnosticSeverity::Warning => {
            diagnostic.severity() == PipelineDiagnosticSeverity::Warning
        }
        InspectDiagnosticSeverity::Info => {
            diagnostic.severity() == PipelineDiagnosticSeverity::Info
        }
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

fn is_in_page(index: usize, limit: usize, offset: usize) -> bool {
    index >= offset && index < offset.saturating_add(limit)
}

fn normalize_file_filter(file: Option<&str>) -> Option<String> {
    file.map(|value| value.replace('\\', "/"))
}
