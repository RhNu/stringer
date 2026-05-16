use stringer_workspace_core::GlobalConfigSource;

use crate::adapt::adapt_import_with_global_config_source;
use crate::dto::{
    AdaptImportRequest, AdaptImportResponse, KnowledgeAnnotateRequest,
    KnowledgeIndexRebuildRequest, KnowledgeIndexRebuildResponse, KnowledgeLookupRequest,
    KnowledgeLookupResponse, KnowledgeOperationResponse, KnowledgeTermDeleteRequest,
    KnowledgeTermEditResponse, KnowledgeTermUpsertRequest, KnowledgeTermsEditResponse,
    KnowledgeValidateRequest, WorkspaceBatchApplyRequest, WorkspaceBatchApplyResponse,
    WorkspaceBatchClaimRequest, WorkspaceBatchClaimResponse, WorkspaceBatchCountRequest,
    WorkspaceBatchCountResponse, WorkspaceBatchReleaseRequest, WorkspaceBatchReleaseResponse,
    WorkspaceFinalizeRequest, WorkspaceFinalizeResponse, WorkspaceInspectBatchRequest,
    WorkspaceInspectBatchResponse, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectDiagnosticsResponse, WorkspaceInspectEntriesRequest,
    WorkspaceInspectEntriesResponse, WorkspaceInspectEntryRequest, WorkspaceInspectEntryResponse,
    WorkspaceInspectFilesRequest, WorkspaceInspectFilesResponse, WorkspaceNormalizeRequest,
    WorkspaceNormalizeResponse, WorkspaceOpenRequest, WorkspaceOpenResponse,
};
use crate::error::AppError;
use crate::knowledge::{
    knowledge_annotate_with_global_config_source,
    knowledge_index_rebuild_with_global_config_source, knowledge_lookup_with_global_config_source,
    knowledge_term_delete, knowledge_term_upsert, knowledge_validate_with_global_config_source,
};
use crate::workspace::{
    workspace_batch_apply, workspace_batch_claim, workspace_batch_count, workspace_batch_release,
    workspace_finalize, workspace_inspect_batch, workspace_inspect_diagnostics,
    workspace_inspect_entries, workspace_inspect_entry, workspace_inspect_files,
    workspace_normalize, workspace_open_with_global_config_source,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringerApp {
    global_config_source: GlobalConfigSource,
}

impl Default for StringerApp {
    fn default() -> Self {
        Self::with_global_config_source(GlobalConfigSource::Production)
    }
}

impl StringerApp {
    pub fn with_global_config_source(global_config_source: GlobalConfigSource) -> Self {
        Self {
            global_config_source,
        }
    }

    pub async fn workspace_open(
        &self,
        request: WorkspaceOpenRequest,
    ) -> Result<WorkspaceOpenResponse, AppError> {
        workspace_open_with_global_config_source(request, &self.global_config_source).await
    }

    pub async fn workspace_finalize(
        &self,
        request: WorkspaceFinalizeRequest,
    ) -> Result<WorkspaceFinalizeResponse, AppError> {
        workspace_finalize(request).await
    }

    pub fn workspace_batch_count(
        &self,
        request: WorkspaceBatchCountRequest,
    ) -> Result<WorkspaceBatchCountResponse, AppError> {
        workspace_batch_count(request)
    }

    pub fn workspace_batch_claim(
        &self,
        request: WorkspaceBatchClaimRequest,
    ) -> Result<WorkspaceBatchClaimResponse, AppError> {
        workspace_batch_claim(request)
    }

    pub fn workspace_batch_apply(
        &self,
        request: WorkspaceBatchApplyRequest,
    ) -> Result<WorkspaceBatchApplyResponse, AppError> {
        workspace_batch_apply(request)
    }

    pub fn workspace_batch_release(
        &self,
        request: WorkspaceBatchReleaseRequest,
    ) -> Result<WorkspaceBatchReleaseResponse, AppError> {
        workspace_batch_release(request)
    }

    pub fn workspace_normalize(
        &self,
        request: WorkspaceNormalizeRequest,
    ) -> Result<WorkspaceNormalizeResponse, AppError> {
        workspace_normalize(request)
    }

    pub fn workspace_inspect_files(
        &self,
        request: WorkspaceInspectFilesRequest,
    ) -> Result<WorkspaceInspectFilesResponse, AppError> {
        workspace_inspect_files(request)
    }

    pub fn workspace_inspect_entries(
        &self,
        request: WorkspaceInspectEntriesRequest,
    ) -> Result<WorkspaceInspectEntriesResponse, AppError> {
        workspace_inspect_entries(request)
    }

    pub fn workspace_inspect_entry(
        &self,
        request: WorkspaceInspectEntryRequest,
    ) -> Result<WorkspaceInspectEntryResponse, AppError> {
        workspace_inspect_entry(request)
    }

    pub fn workspace_inspect_batch(
        &self,
        request: WorkspaceInspectBatchRequest,
    ) -> Result<WorkspaceInspectBatchResponse, AppError> {
        workspace_inspect_batch(request)
    }

    pub fn workspace_inspect_diagnostics(
        &self,
        request: WorkspaceInspectDiagnosticsRequest,
    ) -> Result<WorkspaceInspectDiagnosticsResponse, AppError> {
        workspace_inspect_diagnostics(request)
    }

    pub async fn adapt_import(
        &self,
        request: AdaptImportRequest,
    ) -> Result<AdaptImportResponse, AppError> {
        adapt_import_with_global_config_source(request, &self.global_config_source).await
    }

    pub fn knowledge_annotate(
        &self,
        request: KnowledgeAnnotateRequest,
    ) -> Result<KnowledgeOperationResponse, AppError> {
        knowledge_annotate_with_global_config_source(request, &self.global_config_source)
    }

    pub fn knowledge_validate(
        &self,
        request: KnowledgeValidateRequest,
    ) -> Result<KnowledgeOperationResponse, AppError> {
        knowledge_validate_with_global_config_source(request, &self.global_config_source)
    }

    pub fn knowledge_lookup(
        &self,
        request: KnowledgeLookupRequest,
    ) -> Result<KnowledgeLookupResponse, AppError> {
        knowledge_lookup_with_global_config_source(request, &self.global_config_source)
    }

    pub fn knowledge_index_rebuild(
        &self,
        request: KnowledgeIndexRebuildRequest,
    ) -> Result<KnowledgeIndexRebuildResponse, AppError> {
        knowledge_index_rebuild_with_global_config_source(request, &self.global_config_source)
    }

    pub fn knowledge_term_upsert(
        &self,
        request: KnowledgeTermUpsertRequest,
    ) -> Result<KnowledgeTermsEditResponse, AppError> {
        knowledge_term_upsert(request)
    }

    pub fn knowledge_term_delete(
        &self,
        request: KnowledgeTermDeleteRequest,
    ) -> Result<KnowledgeTermEditResponse, AppError> {
        knowledge_term_delete(request)
    }
}
