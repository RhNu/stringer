use std::collections::BTreeMap;

use stringer_workspace_api::{
    ApplyBatchPatchEntry, ApplyBatchPatchOptions, BatchCount, BatchDetailEntry, BatchExportFormat,
    BatchExportOptions, BatchExportSummary, BatchReadEntry, BatchSubmitAction, BatchSubmitEntry,
    BatchSubmitEntryResult, BatchSubmitOptions, BatchSubmitStatus, ClaimBatchOptions, ClaimedBatch,
    CountBatchOptions, ExportTranslationsOptions, ImportTranslationsOptions,
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, NormalizeRuleEncoding,
    NormalizeWarning, NormalizeWorkspaceOptions, NormalizeWorkspaceSummary, ReadBatchDetailOptions,
    ReadBatchOptions, ReleaseBatchOptions, ReleaseBatchSummary, WorkspaceError,
    WorkspaceInspectBatch, WorkspaceInspectDiagnostic, WorkspaceInspectEntry,
    WorkspaceInspectFiles, WorkspaceNormalizeChange, apply_batch_patch, count_batch,
    export_batch_patch, export_translations, import_translations, inspect_workspace_batch,
    inspect_workspace_diagnostics, inspect_workspace_entries, inspect_workspace_entry,
    inspect_workspace_files, normalize_workspace as normalize_workspace_api, read_batch,
    read_batch_detail, release_batch, submit_batch,
};
use stringer_workspace_core::GlobalConfigSource;

use crate::dto::{
    InspectDiagnosticSeverityInput, InspectEntryStatusInput, WorkspaceBatchApplyRequest,
    WorkspaceBatchApplyResponse, WorkspaceBatchClaimRequest, WorkspaceBatchClaimResponse,
    WorkspaceBatchCountRequest, WorkspaceBatchCountResponse, WorkspaceBatchDetailEntryResponse,
    WorkspaceBatchDetailRequest, WorkspaceBatchDetailResponse, WorkspaceBatchExportFormatInput,
    WorkspaceBatchExportRequest, WorkspaceBatchExportResponse, WorkspaceBatchReadEntryResponse,
    WorkspaceBatchReadRequest, WorkspaceBatchReadResponse, WorkspaceBatchReleaseRequest,
    WorkspaceBatchReleaseResponse, WorkspaceBatchScope, WorkspaceBatchSubmitActionInput,
    WorkspaceBatchSubmitEntry, WorkspaceBatchSubmitEntryResultResponse,
    WorkspaceBatchSubmitRequest, WorkspaceBatchSubmitResponse, WorkspaceBatchSubmitStatusResponse,
    WorkspaceFinalizeRequest, WorkspaceFinalizeResponse, WorkspaceInspectBatchRequest,
    WorkspaceInspectBatchResponse, WorkspaceInspectDiagnosticResponse,
    WorkspaceInspectDiagnosticsRequest, WorkspaceInspectDiagnosticsResponse,
    WorkspaceInspectEntriesRequest, WorkspaceInspectEntriesResponse, WorkspaceInspectEntryRequest,
    WorkspaceInspectEntryResponse, WorkspaceInspectEntrySummaryResponse,
    WorkspaceInspectFileResponse, WorkspaceInspectFilesRequest, WorkspaceInspectFilesResponse,
    WorkspaceNormalizeChangeResponse, WorkspaceNormalizeEncodingInput, WorkspaceNormalizeRequest,
    WorkspaceNormalizeResponse, WorkspaceNormalizeWarningResponse, WorkspaceOpenRequest,
    WorkspaceOpenResponse,
};
use crate::error::{AppError, serialize_value};
use crate::paths::{default_output_path, path, workspace_or_current};
use crate::settings::load_settings_for_workspace;

pub async fn workspace_open(
    request: WorkspaceOpenRequest,
) -> Result<WorkspaceOpenResponse, AppError> {
    workspace_open_with_global_config_source(request, &GlobalConfigSource::Production).await
}

pub(crate) async fn workspace_open_with_global_config_source(
    request: WorkspaceOpenRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<WorkspaceOpenResponse, AppError> {
    let workspace = workspace_or_current(request.workspace)?;
    let settings = load_settings_for_workspace(&workspace, request.settings, global_config_source)?;
    let summary = export_translations(ExportTranslationsOptions {
        source_root: path(request.source_root),
        workspace,
        settings,
        force: request.force,
    })
    .await?;
    Ok(WorkspaceOpenResponse {
        entries: summary.entries,
    })
}

pub async fn workspace_finalize(
    request: WorkspaceFinalizeRequest,
) -> Result<WorkspaceFinalizeResponse, AppError> {
    let workspace = workspace_or_current(request.workspace)?;
    let output = request
        .output
        .map(path)
        .unwrap_or_else(|| default_output_path(&workspace));
    let summary = import_translations(ImportTranslationsOptions {
        workspace,
        source_root: request.source_root.map(path),
        output,
    })
    .await?;
    Ok(WorkspaceFinalizeResponse {
        applied_entries: summary.applied_entries,
        written_files: summary.written_files,
    })
}

pub fn workspace_batch_count(
    request: WorkspaceBatchCountRequest,
) -> Result<WorkspaceBatchCountResponse, AppError> {
    Ok(batch_count_response(count_batch(CountBatchOptions {
        workspace: workspace_or_current(request.workspace)?,
        file: request.file,
    })?))
}

pub fn workspace_batch_claim(
    request: WorkspaceBatchClaimRequest,
) -> Result<WorkspaceBatchClaimResponse, AppError> {
    claimed_batch_response(stringer_workspace_api::claim_batch(ClaimBatchOptions {
        workspace: workspace_or_current(request.workspace)?,
        file: request.file,
        limit: request.limit,
    })?)
}

pub fn workspace_batch_apply(
    request: WorkspaceBatchApplyRequest,
) -> Result<WorkspaceBatchApplyResponse, AppError> {
    let summary = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        entries: request
            .entries
            .into_iter()
            .map(|entry| ApplyBatchPatchEntry {
                id: entry.id,
                translation: entry.translation,
                skip: entry.skip,
                skip_reason: entry.skip_reason,
            })
            .collect(),
    })?;
    Ok(WorkspaceBatchApplyResponse {
        applied_entries: summary.applied_entries,
        remaining_entries: summary.remaining_entries,
    })
}

pub fn workspace_batch_release(
    request: WorkspaceBatchReleaseRequest,
) -> Result<WorkspaceBatchReleaseResponse, AppError> {
    Ok(release_batch_response(release_batch(
        ReleaseBatchOptions {
            workspace: workspace_or_current(request.workspace)?,
            batch_id: request.batch_id,
        },
    )?))
}

pub fn workspace_batch_read(
    request: WorkspaceBatchReadRequest,
) -> Result<WorkspaceBatchReadResponse, AppError> {
    let read = read_batch(ReadBatchOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        offset: request.offset,
        limit: request.limit,
    })?;
    Ok(WorkspaceBatchReadResponse {
        batch_id: read.batch_id,
        revision: read.revision,
        total_entries: read.total_entries,
        open_entries: read.open_entries,
        offset: read.offset,
        limit: read.limit,
        next_offset: read.next_offset,
        entries: read
            .entries
            .into_iter()
            .map(batch_read_entry_response)
            .collect(),
    })
}

pub fn workspace_batch_detail(
    request: WorkspaceBatchDetailRequest,
) -> Result<WorkspaceBatchDetailResponse, AppError> {
    let detail = read_batch_detail(ReadBatchDetailOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        keys: request.keys,
    })?;
    Ok(WorkspaceBatchDetailResponse {
        batch_id: detail.batch_id,
        revision: detail.revision,
        entries: detail
            .entries
            .into_iter()
            .map(batch_detail_entry_response)
            .collect::<Result<_, _>>()?,
    })
}

pub fn workspace_batch_submit(
    request: WorkspaceBatchSubmitRequest,
) -> Result<WorkspaceBatchSubmitResponse, AppError> {
    let summary = submit_batch(BatchSubmitOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        revision: request.revision,
        entries: request
            .entries
            .into_iter()
            .map(batch_submit_entry)
            .collect(),
    })?;
    Ok(WorkspaceBatchSubmitResponse {
        batch_id: summary.batch_id,
        revision: summary.revision,
        applied_entries: summary.applied_entries,
        ignored_entries: summary.ignored_entries,
        rejected_entries: summary.rejected_entries,
        remaining_entries: summary.remaining_entries,
        next_read_offset: summary.next_read_offset,
        results: summary
            .results
            .into_iter()
            .map(batch_submit_result_response)
            .collect(),
    })
}

pub fn workspace_batch_export(
    request: WorkspaceBatchExportRequest,
) -> Result<WorkspaceBatchExportResponse, AppError> {
    let summary = export_batch_patch(BatchExportOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        out: request.out.map(path),
        format: request.format.into(),
    })?;
    Ok(batch_export_response(summary))
}

pub fn workspace_normalize(
    request: WorkspaceNormalizeRequest,
) -> Result<WorkspaceNormalizeResponse, AppError> {
    Ok(normalize_response(normalize_workspace_api(
        NormalizeWorkspaceOptions {
            workspace: workspace_or_current(request.workspace)?,
            rules: path(request.rules),
            file: request.file,
            apply: request.apply,
            encoding: request.encoding.into(),
            limit: request.limit,
        },
    )?))
}

pub fn workspace_inspect_files(
    request: WorkspaceInspectFilesRequest,
) -> Result<WorkspaceInspectFilesResponse, AppError> {
    inspect_files_response(inspect_workspace_files(InspectWorkspaceFilesOptions {
        workspace: workspace_or_current(request.workspace)?,
    })?)
}

pub fn workspace_inspect_entries(
    request: WorkspaceInspectEntriesRequest,
) -> Result<WorkspaceInspectEntriesResponse, AppError> {
    let inspected = inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: workspace_or_current(request.workspace)?,
        file: request.file,
        status: request.status.into(),
        limit: request.limit,
        offset: request.offset,
    })?;
    Ok(WorkspaceInspectEntriesResponse {
        total: inspected.total,
        entries: inspected
            .entries
            .into_iter()
            .map(inspect_entry_summary_response)
            .collect(),
    })
}

pub fn workspace_inspect_entry(
    request: WorkspaceInspectEntryRequest,
) -> Result<WorkspaceInspectEntryResponse, AppError> {
    inspect_entry_response(inspect_workspace_entry(InspectWorkspaceEntryOptions {
        workspace: workspace_or_current(request.workspace)?,
        id: request.id,
    })?)
}

pub fn workspace_inspect_batch(
    request: WorkspaceInspectBatchRequest,
) -> Result<WorkspaceInspectBatchResponse, AppError> {
    inspect_batch_response(inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        offset: request.offset,
        limit: request.limit,
    })?)
}

pub fn workspace_inspect_diagnostics(
    request: WorkspaceInspectDiagnosticsRequest,
) -> Result<WorkspaceInspectDiagnosticsResponse, AppError> {
    let inspected = inspect_workspace_diagnostics(InspectWorkspaceDiagnosticsOptions {
        workspace: workspace_or_current(request.workspace)?,
        file: request.file,
        severity: request.severity.into(),
        limit: request.limit,
        offset: request.offset,
    })?;
    Ok(WorkspaceInspectDiagnosticsResponse {
        total: inspected.total,
        diagnostics: inspected
            .diagnostics
            .into_iter()
            .map(inspect_diagnostic_response)
            .collect::<Result<_, _>>()?,
    })
}

pub fn workspace_upgrade_unsupported(workspace: String) -> AppError {
    WorkspaceError::InvalidTranslationPackagePath {
        path: workspace,
        message: "workspace upgrade is not implemented; recreate/open a v3 workspace instead"
            .to_string(),
    }
    .into()
}

fn batch_count_response(count: BatchCount) -> WorkspaceBatchCountResponse {
    WorkspaceBatchCountResponse {
        total: count.total,
        claimable: count.claimable,
        empty: count.empty,
        memory_prefilled: count.memory_prefilled,
        translated: count.translated,
        skipped: count.skipped,
        claimed: count.claimed,
        diagnostics: count.diagnostics,
    }
}

fn claimed_batch_response(claimed: ClaimedBatch) -> Result<WorkspaceBatchClaimResponse, AppError> {
    Ok(WorkspaceBatchClaimResponse {
        batch_id: claimed.batch_id,
        revision: claimed.revision,
        claimed_entries: claimed.claimed_entries,
        remaining_claimable: claimed.remaining_claimable,
        scope: WorkspaceBatchScope {
            file: claimed.scope.file,
        },
    })
}

fn release_batch_response(summary: ReleaseBatchSummary) -> WorkspaceBatchReleaseResponse {
    WorkspaceBatchReleaseResponse {
        released_entries: summary.released_entries,
    }
}

fn batch_read_entry_response(entry: BatchReadEntry) -> WorkspaceBatchReadEntryResponse {
    WorkspaceBatchReadEntryResponse {
        key: entry.key,
        source: entry.source,
        current_translation: entry.current_translation,
        origin: entry.origin,
        context_label: entry.context_label,
        hint_count: entry.hint_count,
        diagnostic_count: entry.diagnostic_count,
        diagnostic_codes: entry.diagnostic_codes,
    }
}

fn batch_detail_entry_response(
    entry: BatchDetailEntry,
) -> Result<WorkspaceBatchDetailEntryResponse, AppError> {
    Ok(WorkspaceBatchDetailEntryResponse {
        key: entry.key,
        file: entry.file,
        id: entry.id,
        source: entry.source,
        translation: entry.translation,
        translation_meta: entry
            .translation_meta
            .map(|meta| serialize_value("translation_meta", meta))
            .transpose()?,
        context: entry.context,
        hints: entry
            .hints
            .into_iter()
            .map(|hint| serialize_value("batch detail hint", hint))
            .collect::<Result<_, _>>()?,
        diagnostics: entry
            .diagnostics
            .into_iter()
            .map(|diagnostic| serialize_value("batch detail diagnostic", diagnostic))
            .collect::<Result<_, _>>()?,
        claimed_by: entry.claimed_by,
    })
}

fn batch_submit_entry(entry: WorkspaceBatchSubmitEntry) -> BatchSubmitEntry {
    BatchSubmitEntry {
        key: entry.key,
        action: entry.action.into(),
        translation: entry.translation,
        skip_reason: entry.skip_reason,
    }
}

fn batch_submit_result_response(
    result: BatchSubmitEntryResult,
) -> WorkspaceBatchSubmitEntryResultResponse {
    WorkspaceBatchSubmitEntryResultResponse {
        key: result.key,
        status: result.status.into(),
        message: result.message,
    }
}

fn batch_export_response(summary: BatchExportSummary) -> WorkspaceBatchExportResponse {
    WorkspaceBatchExportResponse {
        path: summary.path,
        format: summary.format.into(),
        entries: summary.entries,
    }
}

fn normalize_response(summary: NormalizeWorkspaceSummary) -> WorkspaceNormalizeResponse {
    WorkspaceNormalizeResponse {
        scanned_entries: summary.scanned_entries,
        changed_entries: summary.changed_entries,
        total_replacements: summary.total_replacements,
        skipped_claimed: summary.skipped_claimed,
        skipped_placeholder_risk: summary.skipped_placeholder_risk,
        warnings: summary
            .warnings
            .into_iter()
            .map(normalize_warning_response)
            .collect(),
        changes: summary
            .changes
            .into_iter()
            .map(normalize_change_response)
            .collect(),
    }
}

fn normalize_warning_response(warning: NormalizeWarning) -> WorkspaceNormalizeWarningResponse {
    WorkspaceNormalizeWarningResponse {
        code: warning.code,
        message: warning.message,
        line: warning.line,
        search: warning.search,
        replace: warning.replace,
    }
}

fn normalize_change_response(change: WorkspaceNormalizeChange) -> WorkspaceNormalizeChangeResponse {
    WorkspaceNormalizeChangeResponse {
        file: change.file,
        id: change.id,
        source: change.source,
        before: change.before,
        after: change.after,
        replacements: change.replacements,
        rule_ids: change.rule_ids,
        skipped_placeholder_risk: change.skipped_placeholder_risk,
    }
}

fn inspect_files_response(
    inspected: WorkspaceInspectFiles,
) -> Result<WorkspaceInspectFilesResponse, AppError> {
    Ok(WorkspaceInspectFilesResponse {
        files: inspected
            .files
            .into_iter()
            .map(|file| WorkspaceInspectFileResponse {
                path: file.path,
                kind: file.kind,
                asset_path: file.asset_path,
                group: file.group,
            })
            .collect(),
    })
}

fn inspect_batch_response(
    inspected: WorkspaceInspectBatch,
) -> Result<WorkspaceInspectBatchResponse, AppError> {
    Ok(WorkspaceInspectBatchResponse {
        batch_id: inspected.batch_id,
        total: inspected.total,
        offset: inspected.offset,
        limit: inspected.limit,
        next_offset: inspected.next_offset,
        entries: inspected
            .entries
            .into_iter()
            .map(inspect_entry_response)
            .collect::<Result<_, _>>()?,
    })
}

fn inspect_entry_response(
    entry: WorkspaceInspectEntry,
) -> Result<WorkspaceInspectEntryResponse, AppError> {
    Ok(WorkspaceInspectEntryResponse {
        file: entry.file,
        id: entry.id,
        source: entry.source,
        translation: entry.translation,
        translation_meta: entry
            .translation_meta
            .map(|meta| serialize_value("translation_meta", meta))
            .transpose()?,
        context: entry.context,
        hints: entry
            .hints
            .into_iter()
            .map(|hint| serialize_value("inspect hint", hint))
            .collect::<Result<_, _>>()?,
        diagnostics: entry
            .diagnostics
            .into_iter()
            .map(|diagnostic| serialize_value("inspect diagnostic", diagnostic))
            .collect::<Result<_, _>>()?,
        claimed_by: entry.claimed_by,
    })
}

fn inspect_entry_summary_response(
    entry: WorkspaceInspectEntry,
) -> WorkspaceInspectEntrySummaryResponse {
    WorkspaceInspectEntrySummaryResponse {
        file: entry.file.clone(),
        id: entry.id,
        source: entry.source,
        current_translation: entry.translation,
        origin: entry
            .translation_meta
            .as_ref()
            .and_then(|meta| meta.origin.clone()),
        context_label: context_label(&entry.file, &entry.context),
        hint_count: entry.hints.len(),
        diagnostic_count: entry.diagnostics.len(),
        diagnostic_codes: entry
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code().to_string())
            .collect(),
        claimed_by: entry.claimed_by,
    }
}

fn inspect_diagnostic_response(
    entry: WorkspaceInspectDiagnostic,
) -> Result<WorkspaceInspectDiagnosticResponse, AppError> {
    Ok(WorkspaceInspectDiagnosticResponse {
        entry_id: entry.entry_id,
        file: entry.file.clone(),
        source: entry.source,
        current_translation: entry.translation,
        context_label: context_label(&entry.file, &entry.context),
        code: entry.diagnostic.code().to_string(),
        severity: entry.diagnostic.severity().as_str().to_string(),
        message: entry.diagnostic.message().to_string(),
    })
}

fn context_label(file: &str, context: &BTreeMap<String, String>) -> String {
    if file.starts_with("entries/plugin/") {
        return label_from_keys("plugin", context, &["record_type", "subrecord", "form_id"]);
    }
    if file.starts_with("entries/pex/") {
        return label_from_keys(
            "pex",
            context,
            &["object", "state", "function", "opcode", "operand"],
        );
    }
    if file.starts_with("entries/scaleform/") {
        return label_from_keys("scaleform", context, &["key"]);
    }
    label_from_keys("entry", context, &["record_type", "subrecord", "key"])
}

fn label_from_keys(prefix: &str, context: &BTreeMap<String, String>, keys: &[&str]) -> String {
    let parts = keys
        .iter()
        .filter_map(|key| context.get(*key))
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if parts.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {}", parts.join(" "))
    }
}

impl From<InspectEntryStatusInput> for InspectEntryStatus {
    fn from(value: InspectEntryStatusInput) -> Self {
        match value {
            InspectEntryStatusInput::All => Self::All,
            InspectEntryStatusInput::Empty => Self::Empty,
            InspectEntryStatusInput::Memory => Self::Memory,
            InspectEntryStatusInput::Translated => Self::Translated,
            InspectEntryStatusInput::Skipped => Self::Skipped,
            InspectEntryStatusInput::Claimed => Self::Claimed,
            InspectEntryStatusInput::Diagnostic => Self::Diagnostic,
        }
    }
}

impl From<InspectDiagnosticSeverityInput> for InspectDiagnosticSeverity {
    fn from(value: InspectDiagnosticSeverityInput) -> Self {
        match value {
            InspectDiagnosticSeverityInput::All => Self::All,
            InspectDiagnosticSeverityInput::Error => Self::Error,
            InspectDiagnosticSeverityInput::Warning => Self::Warning,
            InspectDiagnosticSeverityInput::Info => Self::Info,
        }
    }
}

impl From<WorkspaceNormalizeEncodingInput> for NormalizeRuleEncoding {
    fn from(value: WorkspaceNormalizeEncodingInput) -> Self {
        match value {
            WorkspaceNormalizeEncodingInput::Auto => Self::Auto,
            WorkspaceNormalizeEncodingInput::Utf8 => Self::Utf8,
            WorkspaceNormalizeEncodingInput::Cp936 => Self::Cp936,
        }
    }
}

impl From<WorkspaceBatchSubmitActionInput> for BatchSubmitAction {
    fn from(value: WorkspaceBatchSubmitActionInput) -> Self {
        match value {
            WorkspaceBatchSubmitActionInput::Translate => Self::Translate,
            WorkspaceBatchSubmitActionInput::Skip => Self::Skip,
            WorkspaceBatchSubmitActionInput::Pending => Self::Pending,
        }
    }
}

impl From<BatchSubmitStatus> for WorkspaceBatchSubmitStatusResponse {
    fn from(value: BatchSubmitStatus) -> Self {
        match value {
            BatchSubmitStatus::Applied => Self::Applied,
            BatchSubmitStatus::Ignored => Self::Ignored,
            BatchSubmitStatus::Rejected => Self::Rejected,
        }
    }
}

impl From<WorkspaceBatchExportFormatInput> for BatchExportFormat {
    fn from(value: WorkspaceBatchExportFormatInput) -> Self {
        match value {
            WorkspaceBatchExportFormatInput::Json => Self::Json,
            WorkspaceBatchExportFormatInput::Csv => Self::Csv,
        }
    }
}

impl From<BatchExportFormat> for WorkspaceBatchExportFormatInput {
    fn from(value: BatchExportFormat) -> Self {
        match value {
            BatchExportFormat::Json => Self::Json,
            BatchExportFormat::Csv => Self::Csv,
        }
    }
}
