#![forbid(unsafe_code)]

mod schema;

use rmcp::{
    ErrorData, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    tool, tool_router,
};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use stringer_app::{
    AppError, adapt_import, knowledge_annotate, knowledge_index_rebuild, knowledge_lookup,
    knowledge_validate, workspace_batch_apply, workspace_batch_claim, workspace_batch_count,
    workspace_batch_release, workspace_finalize, workspace_open,
};

pub use schema::*;

#[derive(Debug, Clone, Default)]
pub struct StringerMcp;

#[tool_router(server_handler)]
impl StringerMcp {
    #[tool(
        name = "workspace_open",
        description = "Open a Stringer translation workspace from a Bethesda mod root."
    )]
    pub async fn workspace_open(
        &self,
        Parameters(request): Parameters<WorkspaceOpenParams>,
    ) -> Result<Json<WorkspaceOpenResult>, ErrorData> {
        app_json(workspace_open(app_request(request)?).await)
    }

    #[tool(
        name = "workspace_finalize",
        description = "Finalize a Stringer translation workspace into an override directory."
    )]
    pub async fn workspace_finalize(
        &self,
        Parameters(request): Parameters<WorkspaceFinalizeParams>,
    ) -> Result<Json<WorkspaceFinalizeResult>, ErrorData> {
        app_json(workspace_finalize(app_request(request)?).await)
    }

    #[tool(
        name = "workspace_batch_count",
        description = "Count translation entries, claimed entries, and diagnostics in a workspace."
    )]
    pub async fn workspace_batch_count(
        &self,
        Parameters(request): Parameters<WorkspaceBatchCountParams>,
    ) -> Result<Json<WorkspaceBatchCountResult>, ErrorData> {
        app_json(workspace_batch_count(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_claim",
        description = "Claim a batch of eligible translation entries for agent work."
    )]
    pub async fn workspace_batch_claim(
        &self,
        Parameters(request): Parameters<WorkspaceBatchClaimParams>,
    ) -> Result<Json<WorkspaceBatchClaimResult>, ErrorData> {
        app_json(workspace_batch_claim(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_apply",
        description = "Apply translated entries for a previously claimed batch."
    )]
    pub async fn workspace_batch_apply(
        &self,
        Parameters(request): Parameters<WorkspaceBatchApplyParams>,
    ) -> Result<Json<WorkspaceBatchApplyResult>, ErrorData> {
        app_json(workspace_batch_apply(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_release",
        description = "Release a claimed translation batch without applying translations."
    )]
    pub async fn workspace_batch_release(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReleaseParams>,
    ) -> Result<Json<WorkspaceBatchReleaseResult>, ErrorData> {
        app_json(workspace_batch_release(app_request(request)?))
    }

    #[tool(
        name = "adapt_import",
        description = "Import an external translation resource as Stringer memory JSONL."
    )]
    pub async fn adapt_import(
        &self,
        Parameters(request): Parameters<AdaptImportParams>,
    ) -> Result<Json<AdaptImportResult>, ErrorData> {
        app_json(adapt_import(app_request(request)?).await)
    }

    #[tool(
        name = "knowledge_annotate",
        description = "Annotate workspace entries with terminology, memory, and diagnostics."
    )]
    pub async fn knowledge_annotate(
        &self,
        Parameters(request): Parameters<KnowledgeAnnotateParams>,
    ) -> Result<Json<KnowledgeOperationResult>, ErrorData> {
        app_json(knowledge_annotate(app_request(request)?))
    }

    #[tool(
        name = "knowledge_validate",
        description = "Validate workspace translations and rewrite diagnostics."
    )]
    pub async fn knowledge_validate(
        &self,
        Parameters(request): Parameters<KnowledgeValidateParams>,
    ) -> Result<Json<KnowledgeOperationResult>, ErrorData> {
        app_json(knowledge_validate(app_request(request)?))
    }

    #[tool(
        name = "knowledge_lookup",
        description = "Search Stringer terminology and translation memory."
    )]
    pub async fn knowledge_lookup(
        &self,
        Parameters(request): Parameters<KnowledgeLookupParams>,
    ) -> Result<Json<KnowledgeLookupResult>, ErrorData> {
        app_json(knowledge_lookup(app_request(request)?))
    }

    #[tool(
        name = "knowledge_index_rebuild",
        description = "Rebuild the derived SQLite knowledge index for a project root."
    )]
    pub async fn knowledge_index_rebuild(
        &self,
        Parameters(request): Parameters<KnowledgeIndexRebuildParams>,
    ) -> Result<Json<KnowledgeIndexRebuildResult>, ErrorData> {
        app_json(knowledge_index_rebuild(app_request(request)?))
    }
}

pub async fn serve_stdio() -> Result<(), Box<dyn std::error::Error>> {
    StringerMcp
        .serve(rmcp::transport::stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}

fn app_request<T, U>(request: T) -> Result<U, ErrorData>
where
    T: Serialize,
    U: DeserializeOwned,
{
    let value = serde_json::to_value(request).map_err(|source| {
        ErrorData::invalid_params(
            "failed to serialize Stringer MCP request",
            Some(json!({
                "code": "mcp.serialize_request",
                "message": source.to_string(),
                "details": {},
            })),
        )
    })?;
    serde_json::from_value(value).map_err(|source| {
        ErrorData::invalid_params(
            "failed to map Stringer MCP request",
            Some(json!({
                "code": "mcp.map_request",
                "message": source.to_string(),
                "details": {},
            })),
        )
    })
}

fn app_json<T, U>(result: Result<T, AppError>) -> Result<Json<U>, ErrorData>
where
    T: Serialize,
    U: DeserializeOwned + Serialize + JsonSchema,
{
    let value = serde_json::to_value(result.map_err(app_error)?).map_err(|source| {
        ErrorData::internal_error(
            "failed to serialize Stringer MCP result",
            Some(json!({
                "code": "mcp.serialize_result",
                "message": source.to_string(),
                "details": {},
            })),
        )
    })?;
    let output = serde_json::from_value(value).map_err(|source| {
        ErrorData::internal_error(
            "failed to map Stringer MCP result",
            Some(json!({
                "code": "mcp.map_result",
                "message": source.to_string(),
                "details": {},
            })),
        )
    })?;
    Ok(Json(output))
}

fn app_error(error: AppError) -> ErrorData {
    let message = error.to_string();
    ErrorData::internal_error(
        message.clone(),
        Some(json!({
            "code": error.code(),
            "message": message,
            "details": error.details(),
        })),
    )
}
