#![forbid(unsafe_code)]

mod schema;

use std::{any::Any, sync::Arc};

use rmcp::{
    ErrorData, ServiceExt,
    handler::server::wrapper::{Json, Parameters},
    model::JsonObject,
    tool, tool_router,
};
use schemars::{JsonSchema, generate::SchemaSettings, transform::Transform};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use stringer_app::{
    AppError, adapt_import, knowledge_annotate, knowledge_index_rebuild, knowledge_lookup,
    knowledge_term_delete, knowledge_term_upsert, knowledge_validate, workspace_batch_apply,
    workspace_batch_claim, workspace_batch_count, workspace_batch_release, workspace_finalize,
    workspace_open,
};

pub use schema::*;

#[derive(Debug, Clone, Default)]
pub struct StringerMcp;

#[tool_router(server_handler)]
impl StringerMcp {
    #[tool(
        name = "workspace_open",
        description = "Open a translation workspace from a Bethesda mod root.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceOpenParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceOpenResult>()
    )]
    pub async fn workspace_open(
        &self,
        Parameters(request): Parameters<WorkspaceOpenParams>,
    ) -> Result<Json<WorkspaceOpenResult>, ErrorData> {
        app_json(workspace_open(app_request(request)?).await)
    }

    #[tool(
        name = "workspace_finalize",
        description = "Finalize a translation workspace into an override directory.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceFinalizeParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceFinalizeResult>()
    )]
    pub async fn workspace_finalize(
        &self,
        Parameters(request): Parameters<WorkspaceFinalizeParams>,
    ) -> Result<Json<WorkspaceFinalizeResult>, ErrorData> {
        app_json(workspace_finalize(app_request(request)?).await)
    }

    #[tool(
        name = "workspace_batch_count",
        description = "Count translation rows, claims, and diagnostics in a workspace.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchCountParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchCountResult>()
    )]
    pub async fn workspace_batch_count(
        &self,
        Parameters(request): Parameters<WorkspaceBatchCountParams>,
    ) -> Result<Json<WorkspaceBatchCountResult>, ErrorData> {
        app_json(workspace_batch_count(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_claim",
        description = "Claim eligible translation rows for agent work.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchClaimParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchClaimResult>()
    )]
    pub async fn workspace_batch_claim(
        &self,
        Parameters(request): Parameters<WorkspaceBatchClaimParams>,
    ) -> Result<Json<WorkspaceBatchClaimResult>, ErrorData> {
        app_json(workspace_batch_claim(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_apply",
        description = "Apply translations for a claimed batch.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchApplyParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchApplyResult>()
    )]
    pub async fn workspace_batch_apply(
        &self,
        Parameters(request): Parameters<WorkspaceBatchApplyParams>,
    ) -> Result<Json<WorkspaceBatchApplyResult>, ErrorData> {
        app_json(workspace_batch_apply(app_request(request)?))
    }

    #[tool(
        name = "workspace_batch_release",
        description = "Release a claimed batch without applying translations.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchReleaseParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchReleaseResult>()
    )]
    pub async fn workspace_batch_release(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReleaseParams>,
    ) -> Result<Json<WorkspaceBatchReleaseResult>, ErrorData> {
        app_json(workspace_batch_release(app_request(request)?))
    }

    #[tool(
        name = "adapt_import",
        description = "Import an external translation resource as memory JSONL.",
        input_schema = compatible_schema_for_type::<Parameters<AdaptImportParams>>(),
        output_schema = compatible_output_schema_for_type::<AdaptImportResult>()
    )]
    pub async fn adapt_import(
        &self,
        Parameters(request): Parameters<AdaptImportParams>,
    ) -> Result<Json<AdaptImportResult>, ErrorData> {
        app_json(adapt_import(app_request(request)?).await)
    }

    #[tool(
        name = "knowledge_annotate",
        description = "Annotate workspace rows with terminology, memory, and diagnostics.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeAnnotateParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeOperationResult>()
    )]
    pub async fn knowledge_annotate(
        &self,
        Parameters(request): Parameters<KnowledgeAnnotateParams>,
    ) -> Result<Json<KnowledgeOperationResult>, ErrorData> {
        app_json(knowledge_annotate(app_request(request)?))
    }

    #[tool(
        name = "knowledge_validate",
        description = "Validate workspace translations and rewrite diagnostics.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeValidateParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeOperationResult>()
    )]
    pub async fn knowledge_validate(
        &self,
        Parameters(request): Parameters<KnowledgeValidateParams>,
    ) -> Result<Json<KnowledgeOperationResult>, ErrorData> {
        app_json(knowledge_validate(app_request(request)?))
    }

    #[tool(
        name = "knowledge_lookup",
        description = "Search terminology and translation memory.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeLookupParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeLookupResult>()
    )]
    pub async fn knowledge_lookup(
        &self,
        Parameters(request): Parameters<KnowledgeLookupParams>,
    ) -> Result<Json<KnowledgeLookupResult>, ErrorData> {
        app_json(knowledge_lookup(app_request(request)?))
    }

    #[tool(
        name = "knowledge_index_rebuild",
        description = "Rebuild the derived knowledge index for a project root.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeIndexRebuildParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeIndexRebuildResult>()
    )]
    pub async fn knowledge_index_rebuild(
        &self,
        Parameters(request): Parameters<KnowledgeIndexRebuildParams>,
    ) -> Result<Json<KnowledgeIndexRebuildResult>, ErrorData> {
        app_json(knowledge_index_rebuild(app_request(request)?))
    }

    #[tool(
        name = "knowledge_term_upsert",
        description = "Create or replace a project terminology entry.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermUpsertParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermEditResult>()
    )]
    pub async fn knowledge_term_upsert(
        &self,
        Parameters(request): Parameters<KnowledgeTermUpsertParams>,
    ) -> Result<Json<KnowledgeTermEditResult>, ErrorData> {
        app_json(knowledge_term_upsert(app_request(request)?))
    }

    #[tool(
        name = "knowledge_term_delete",
        description = "Delete a project terminology entry by id.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermDeleteParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermEditResult>()
    )]
    pub async fn knowledge_term_delete(
        &self,
        Parameters(request): Parameters<KnowledgeTermDeleteParams>,
    ) -> Result<Json<KnowledgeTermEditResult>, ErrorData> {
        app_json(knowledge_term_delete(app_request(request)?))
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

fn compatible_schema_for_type<T>() -> Arc<JsonObject>
where
    T: JsonSchema + Any,
{
    let generator = SchemaSettings::draft2020_12().into_generator();
    let mut schema = generator.into_root_schema_for::<T>();
    schemars::transform::RestrictFormats::default().transform(&mut schema);
    let value = serde_json::to_value(schema).expect("failed to serialize schema");
    let object = match value {
        serde_json::Value::Object(object) => object,
        _ => panic!("schema serialization produced non-object value"),
    };
    Arc::new(object)
}

fn compatible_output_schema_for_type<T>() -> Arc<JsonObject>
where
    T: JsonSchema + Any,
{
    let schema = compatible_schema_for_type::<T>();
    match schema.get("type") {
        Some(serde_json::Value::String(kind)) if kind == "object" => schema,
        Some(serde_json::Value::String(kind)) => {
            panic!("MCP output schema root type must be object, found {kind}")
        }
        _ => panic!("MCP output schema is missing object root type"),
    }
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
