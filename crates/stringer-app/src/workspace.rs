use stringer_workspace_api::{
    BatchCount, BatchDetailEntry, BatchExportFormat, BatchExportOptions, BatchExportSummary,
    BatchReadEntry, BatchSubmitAction, BatchSubmitEntry, BatchSubmitEntryResult,
    BatchSubmitOptions, BatchSubmitStatus, ClaimBatchOptions, ClaimedBatch, CountBatchOptions,
    ExportTranslationsOptions, ImportTranslationsOptions, InspectDiagnosticSeverity,
    InspectEntryStatus, InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, NormalizeRuleEncoding,
    NormalizeWarning, NormalizeWorkspaceOptions, NormalizeWorkspaceSummary, ReadBatchDetailOptions,
    ReadBatchOptions, ReleaseBatchOptions, ReleaseBatchSummary, WorkspaceInspectDiagnostic,
    WorkspaceInspectEntry, WorkspaceInspectFiles, WorkspaceNormalizeChange, count_batch,
    export_batch_submission, export_translations, import_translations,
    inspect_workspace_diagnostics, inspect_workspace_entries, inspect_workspace_entry,
    inspect_workspace_files, normalize_workspace as normalize_workspace_api, read_batch,
    read_batch_detail, release_batch, submit_batch, workspace_context_label,
};
use stringer_workspace_core::GlobalConfigSource;

use crate::error::{AppError, serialize_value};
use crate::paths::{default_output_path, path, workspace_or_current};
use crate::settings::load_settings_for_workspace;
use stringer_interface::{
    InspectDiagnosticSeverityInput, InspectEntryStatusInput, WorkspaceBatchClaimRequest,
    WorkspaceBatchClaimResponse, WorkspaceBatchCountRequest, WorkspaceBatchCountResponse,
    WorkspaceBatchDetailEntryResponse, WorkspaceBatchDetailRequest, WorkspaceBatchDetailResponse,
    WorkspaceBatchExportFormatInput, WorkspaceBatchExportRequest, WorkspaceBatchExportResponse,
    WorkspaceBatchReadEntryResponse, WorkspaceBatchReadRequest, WorkspaceBatchReadResponse,
    WorkspaceBatchReleaseRequest, WorkspaceBatchReleaseResponse, WorkspaceBatchScope,
    WorkspaceBatchSubmitActionInput, WorkspaceBatchSubmitEntry,
    WorkspaceBatchSubmitEntryResultResponse, WorkspaceBatchSubmitRequest,
    WorkspaceBatchSubmitResponse, WorkspaceBatchSubmitStatusResponse, WorkspaceFinalizeRequest,
    WorkspaceFinalizeResponse, WorkspaceInspectDiagnosticResponse,
    WorkspaceInspectDiagnosticsRequest, WorkspaceInspectDiagnosticsResponse,
    WorkspaceInspectEntriesRequest, WorkspaceInspectEntriesResponse, WorkspaceInspectEntryRequest,
    WorkspaceInspectEntryResponse, WorkspaceInspectEntrySummaryResponse,
    WorkspaceInspectFileResponse, WorkspaceInspectFilesRequest, WorkspaceInspectFilesResponse,
    WorkspaceNormalizeChangeResponse, WorkspaceNormalizeEncodingInput, WorkspaceNormalizeRequest,
    WorkspaceNormalizeResponse, WorkspaceNormalizeWarningResponse, WorkspaceOpenRequest,
    WorkspaceOpenResponse,
};

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
    let summary = export_batch_submission(BatchExportOptions {
        workspace: workspace_or_current(request.workspace)?,
        batch_id: request.batch_id,
        out: request.out.map(path),
        format: batch_export_format(request.format),
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
            encoding: normalize_encoding(request.encoding),
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
        status: inspect_entry_status(request.status),
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

pub fn workspace_inspect_diagnostics(
    request: WorkspaceInspectDiagnosticsRequest,
) -> Result<WorkspaceInspectDiagnosticsResponse, AppError> {
    let inspected = inspect_workspace_diagnostics(InspectWorkspaceDiagnosticsOptions {
        workspace: workspace_or_current(request.workspace)?,
        file: request.file,
        severity: inspect_diagnostic_severity(request.severity),
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
        action: batch_submit_action(entry.action),
        translation: entry.translation,
        skip_reason: entry.skip_reason,
    }
}

fn batch_submit_result_response(
    result: BatchSubmitEntryResult,
) -> WorkspaceBatchSubmitEntryResultResponse {
    WorkspaceBatchSubmitEntryResultResponse {
        key: result.key,
        status: batch_submit_status_response(result.status),
        message: result.message,
    }
}

fn batch_export_response(summary: BatchExportSummary) -> WorkspaceBatchExportResponse {
    WorkspaceBatchExportResponse {
        path: summary.path,
        format: batch_export_format_response(summary.format),
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
        context_label: workspace_context_label(&entry.file, &entry.context),
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
        context_label: workspace_context_label(&entry.file, &entry.context),
        code: entry.diagnostic.code().to_string(),
        severity: entry.diagnostic.severity().as_str().to_string(),
        message: entry.diagnostic.message().to_string(),
    })
}

fn inspect_entry_status(value: InspectEntryStatusInput) -> InspectEntryStatus {
    match value {
        InspectEntryStatusInput::All => InspectEntryStatus::All,
        InspectEntryStatusInput::Empty => InspectEntryStatus::Empty,
        InspectEntryStatusInput::Memory => InspectEntryStatus::Memory,
        InspectEntryStatusInput::Translated => InspectEntryStatus::Translated,
        InspectEntryStatusInput::Skipped => InspectEntryStatus::Skipped,
        InspectEntryStatusInput::Claimed => InspectEntryStatus::Claimed,
        InspectEntryStatusInput::Diagnostic => InspectEntryStatus::Diagnostic,
    }
}

fn inspect_diagnostic_severity(value: InspectDiagnosticSeverityInput) -> InspectDiagnosticSeverity {
    match value {
        InspectDiagnosticSeverityInput::All => InspectDiagnosticSeverity::All,
        InspectDiagnosticSeverityInput::Error => InspectDiagnosticSeverity::Error,
        InspectDiagnosticSeverityInput::Warning => InspectDiagnosticSeverity::Warning,
        InspectDiagnosticSeverityInput::Info => InspectDiagnosticSeverity::Info,
    }
}

fn normalize_encoding(value: WorkspaceNormalizeEncodingInput) -> NormalizeRuleEncoding {
    match value {
        WorkspaceNormalizeEncodingInput::Auto => NormalizeRuleEncoding::Auto,
        WorkspaceNormalizeEncodingInput::Utf8 => NormalizeRuleEncoding::Utf8,
        WorkspaceNormalizeEncodingInput::Cp936 => NormalizeRuleEncoding::Cp936,
    }
}

fn batch_submit_action(value: WorkspaceBatchSubmitActionInput) -> BatchSubmitAction {
    match value {
        WorkspaceBatchSubmitActionInput::Translate => BatchSubmitAction::Translate,
        WorkspaceBatchSubmitActionInput::Skip => BatchSubmitAction::Skip,
        WorkspaceBatchSubmitActionInput::Pending => BatchSubmitAction::Pending,
    }
}

fn batch_submit_status_response(value: BatchSubmitStatus) -> WorkspaceBatchSubmitStatusResponse {
    match value {
        BatchSubmitStatus::Applied => WorkspaceBatchSubmitStatusResponse::Applied,
        BatchSubmitStatus::Ignored => WorkspaceBatchSubmitStatusResponse::Ignored,
        BatchSubmitStatus::Rejected => WorkspaceBatchSubmitStatusResponse::Rejected,
    }
}

fn batch_export_format(value: WorkspaceBatchExportFormatInput) -> BatchExportFormat {
    match value {
        WorkspaceBatchExportFormatInput::Json => BatchExportFormat::Json,
        WorkspaceBatchExportFormatInput::Csv => BatchExportFormat::Csv,
    }
}

fn batch_export_format_response(value: BatchExportFormat) -> WorkspaceBatchExportFormatInput {
    match value {
        BatchExportFormat::Json => WorkspaceBatchExportFormatInput::Json,
        BatchExportFormat::Csv => WorkspaceBatchExportFormatInput::Csv,
    }
}
