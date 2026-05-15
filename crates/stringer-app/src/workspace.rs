use stringer_workspace::{
    ApplyBatchPatchEntry, ApplyBatchPatchOptions, BatchCount, ClaimBatchOptions, ClaimedBatch,
    CountBatchOptions, ExportTranslationsOptions, ImportTranslationsOptions,
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, ReleaseBatchOptions,
    ReleaseBatchSummary, WorkspaceError, WorkspaceInspectBatch, WorkspaceInspectDiagnostic,
    WorkspaceInspectEntry, WorkspaceInspectFiles, WriteTarget, apply_batch_patch, count_batch,
    export_translations, import_translations, inspect_workspace_batch,
    inspect_workspace_diagnostics, inspect_workspace_entries, inspect_workspace_entry,
    inspect_workspace_files, release_batch,
};

use crate::dto::{
    InspectDiagnosticSeverityInput, InspectEntryStatusInput, WorkspaceBatchApplyRequest,
    WorkspaceBatchApplyResponse, WorkspaceBatchClaimEntry, WorkspaceBatchClaimRequest,
    WorkspaceBatchClaimResponse, WorkspaceBatchCountRequest, WorkspaceBatchCountResponse,
    WorkspaceBatchReleaseRequest, WorkspaceBatchReleaseResponse, WorkspaceFinalizeRequest,
    WorkspaceFinalizeResponse, WorkspaceInspectBatchRequest, WorkspaceInspectBatchResponse,
    WorkspaceInspectDiagnosticResponse, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectDiagnosticsResponse, WorkspaceInspectEntriesRequest,
    WorkspaceInspectEntriesResponse, WorkspaceInspectEntryRequest, WorkspaceInspectEntryResponse,
    WorkspaceInspectFileResponse, WorkspaceInspectFilesRequest, WorkspaceInspectFilesResponse,
    WorkspaceOpenRequest, WorkspaceOpenResponse,
};
use crate::error::{AppError, serialize_value};
use crate::paths::path;
use crate::settings::load_settings_for_project;

pub async fn workspace_open(
    request: WorkspaceOpenRequest,
) -> Result<WorkspaceOpenResponse, AppError> {
    let root = path(request.root);
    let settings = load_settings_for_project(&root, request.settings)?;
    let summary = export_translations(ExportTranslationsOptions {
        root,
        out: path(request.workspace),
        settings,
    })
    .await?;
    Ok(WorkspaceOpenResponse {
        entries: summary.entries,
    })
}

pub async fn workspace_finalize(
    request: WorkspaceFinalizeRequest,
) -> Result<WorkspaceFinalizeResponse, AppError> {
    let summary = import_translations(ImportTranslationsOptions {
        root: path(request.root),
        translations: path(request.workspace),
        target: WriteTarget::OverrideDirectory {
            root: path(request.override_root),
        },
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
        workspace: path(request.workspace),
        file: request.file,
    })?))
}

pub fn workspace_batch_claim(
    request: WorkspaceBatchClaimRequest,
) -> Result<WorkspaceBatchClaimResponse, AppError> {
    claimed_batch_response(stringer_workspace::claim_batch(ClaimBatchOptions {
        workspace: path(request.workspace),
        file: request.file,
        limit: request.limit,
    })?)
}

pub fn workspace_batch_apply(
    request: WorkspaceBatchApplyRequest,
) -> Result<WorkspaceBatchApplyResponse, AppError> {
    let summary = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: path(request.workspace),
        batch_id: request.batch_id,
        entries: request
            .entries
            .into_iter()
            .map(|entry| ApplyBatchPatchEntry {
                id: entry.id,
                translation: entry.translation,
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
            workspace: path(request.workspace),
            batch_id: request.batch_id,
        },
    )?))
}

pub fn workspace_inspect_files(
    request: WorkspaceInspectFilesRequest,
) -> Result<WorkspaceInspectFilesResponse, AppError> {
    inspect_files_response(inspect_workspace_files(InspectWorkspaceFilesOptions {
        workspace: path(request.workspace),
    })?)
}

pub fn workspace_inspect_entries(
    request: WorkspaceInspectEntriesRequest,
) -> Result<WorkspaceInspectEntriesResponse, AppError> {
    let inspected = inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: path(request.workspace),
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
            .map(inspect_entry_response)
            .collect::<Result<_, _>>()?,
    })
}

pub fn workspace_inspect_entry(
    request: WorkspaceInspectEntryRequest,
) -> Result<WorkspaceInspectEntryResponse, AppError> {
    inspect_entry_response(inspect_workspace_entry(InspectWorkspaceEntryOptions {
        workspace: path(request.workspace),
        id: request.id,
    })?)
}

pub fn workspace_inspect_batch(
    request: WorkspaceInspectBatchRequest,
) -> Result<WorkspaceInspectBatchResponse, AppError> {
    inspect_batch_response(inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: path(request.workspace),
        batch_id: request.batch_id,
    })?)
}

pub fn workspace_inspect_diagnostics(
    request: WorkspaceInspectDiagnosticsRequest,
) -> Result<WorkspaceInspectDiagnosticsResponse, AppError> {
    let inspected = inspect_workspace_diagnostics(InspectWorkspaceDiagnosticsOptions {
        workspace: path(request.workspace),
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
        empty: count.empty,
        memory_prefilled: count.memory_prefilled,
        translated: count.translated,
        claimed: count.claimed,
        diagnostics: count.diagnostics,
    }
}

fn claimed_batch_response(claimed: ClaimedBatch) -> Result<WorkspaceBatchClaimResponse, AppError> {
    Ok(WorkspaceBatchClaimResponse {
        batch_id: claimed.batch_id,
        entries: claimed
            .entries
            .into_iter()
            .map(|entry| {
                Ok(WorkspaceBatchClaimEntry {
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
                        .map(|hint| serialize_value("batch hint", hint))
                        .collect::<Result<_, _>>()?,
                    diagnostics: entry
                        .diagnostics
                        .into_iter()
                        .map(|diagnostic| serialize_value("batch diagnostic", diagnostic))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, AppError>>()?,
    })
}

fn release_batch_response(summary: ReleaseBatchSummary) -> WorkspaceBatchReleaseResponse {
    WorkspaceBatchReleaseResponse {
        released_entries: summary.released_entries,
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

fn inspect_diagnostic_response(
    entry: WorkspaceInspectDiagnostic,
) -> Result<WorkspaceInspectDiagnosticResponse, AppError> {
    Ok(WorkspaceInspectDiagnosticResponse {
        entry_id: entry.entry_id,
        file: entry.file,
        source: entry.source,
        translation: entry.translation,
        context: entry.context,
        diagnostic: serialize_value("inspect diagnostic", entry.diagnostic)?,
    })
}

impl From<InspectEntryStatusInput> for InspectEntryStatus {
    fn from(value: InspectEntryStatusInput) -> Self {
        match value {
            InspectEntryStatusInput::All => Self::All,
            InspectEntryStatusInput::Empty => Self::Empty,
            InspectEntryStatusInput::Memory => Self::Memory,
            InspectEntryStatusInput::Translated => Self::Translated,
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
