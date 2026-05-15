use stringer_workspace::{
    ApplyBatchPatchEntry, ApplyBatchPatchOptions, BatchCount, ClaimBatchOptions, ClaimedBatch,
    CountBatchOptions, ExportTranslationsOptions, ImportTranslationsOptions, ReleaseBatchOptions,
    ReleaseBatchSummary, WorkspaceError, WriteTarget, apply_batch_patch, count_batch,
    export_translations, import_translations, release_batch,
};

use crate::dto::{
    WorkspaceBatchApplyRequest, WorkspaceBatchApplyResponse, WorkspaceBatchClaimEntry,
    WorkspaceBatchClaimRequest, WorkspaceBatchClaimResponse, WorkspaceBatchCountRequest,
    WorkspaceBatchCountResponse, WorkspaceBatchReleaseRequest, WorkspaceBatchReleaseResponse,
    WorkspaceFinalizeRequest, WorkspaceFinalizeResponse, WorkspaceOpenRequest,
    WorkspaceOpenResponse,
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
