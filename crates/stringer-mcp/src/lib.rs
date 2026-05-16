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
use serde::Serialize;
use serde_json::json;
use stringer_app::{
    AppError, adapt_import, knowledge_annotate, knowledge_index_rebuild, knowledge_lookup,
    knowledge_term_delete, knowledge_term_upsert, knowledge_validate, workspace_batch_claim,
    workspace_batch_count, workspace_batch_detail, workspace_batch_export, workspace_batch_read,
    workspace_batch_release, workspace_batch_submit, workspace_finalize,
    workspace_inspect_diagnostics, workspace_inspect_entries, workspace_inspect_entry,
    workspace_inspect_files, workspace_normalize, workspace_open,
};

pub use schema::*;

#[derive(Debug, Clone, Default)]
pub struct StringerMcp;

#[tool_router(server_handler)]
impl StringerMcp {
    #[tool(
        name = "workspace_open",
        description = "Open a translation workspace from a read-only source root.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceOpenParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceOpenResult>()
    )]
    pub async fn workspace_open(
        &self,
        Parameters(request): Parameters<WorkspaceOpenParams>,
    ) -> Result<Json<WorkspaceOpenResult>, ErrorData> {
        app_json(workspace_open(request).await)
    }

    #[tool(
        name = "workspace_finalize",
        description = "Finalize a translation workspace into an output directory.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceFinalizeParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceFinalizeResult>()
    )]
    pub async fn workspace_finalize(
        &self,
        Parameters(request): Parameters<WorkspaceFinalizeParams>,
    ) -> Result<Json<WorkspaceFinalizeResult>, ErrorData> {
        app_json(workspace_finalize(request).await)
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
        app_json(workspace_batch_count(request))
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
        app_json(workspace_batch_claim(request))
    }

    #[tool(
        name = "workspace_batch_release",
        description = "Release a claimed batch without submitting translations.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchReleaseParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchReleaseResult>()
    )]
    pub async fn workspace_batch_release(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReleaseParams>,
    ) -> Result<Json<WorkspaceBatchReleaseResult>, ErrorData> {
        app_json(workspace_batch_release(request))
    }

    #[tool(
        name = "workspace_batch_read",
        description = "Read compact entries from a claimed batch.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchReadParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchReadResult>()
    )]
    pub async fn workspace_batch_read(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReadParams>,
    ) -> Result<Json<WorkspaceBatchReadResult>, ErrorData> {
        app_json(workspace_batch_read(request))
    }

    #[tool(
        name = "workspace_batch_detail",
        description = "Read full detail for one or more claimed batch keys.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchDetailParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchDetailResult>()
    )]
    pub async fn workspace_batch_detail(
        &self,
        Parameters(request): Parameters<WorkspaceBatchDetailParams>,
    ) -> Result<Json<WorkspaceBatchDetailResult>, ErrorData> {
        app_json(workspace_batch_detail(request))
    }

    #[tool(
        name = "workspace_batch_submit",
        description = "Submit translate, skip, or pending actions for a claimed batch.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchSubmitParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchSubmitResult>()
    )]
    pub async fn workspace_batch_submit(
        &self,
        Parameters(request): Parameters<WorkspaceBatchSubmitParams>,
    ) -> Result<Json<WorkspaceBatchSubmitResult>, ErrorData> {
        app_json(workspace_batch_submit(request))
    }

    #[tool(
        name = "workspace_batch_export",
        description = "Export a claimed batch to an editable JSON or CSV submission file.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchExportParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchExportResult>()
    )]
    pub async fn workspace_batch_export(
        &self,
        Parameters(request): Parameters<WorkspaceBatchExportParams>,
    ) -> Result<Json<WorkspaceBatchExportResult>, ErrorData> {
        app_json(workspace_batch_export(request))
    }

    #[tool(
        name = "workspace_normalize",
        description = "Normalize existing translations with xTranslator Search/Replace rules.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceNormalizeParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceNormalizeResult>()
    )]
    pub async fn workspace_normalize(
        &self,
        Parameters(request): Parameters<WorkspaceNormalizeParams>,
    ) -> Result<Json<WorkspaceNormalizeResult>, ErrorData> {
        app_json(workspace_normalize(request))
    }

    #[tool(
        name = "workspace_inspect_files",
        description = "List workspace entry files without reading raw workspace files directly.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectFilesParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectFilesResult>()
    )]
    pub async fn workspace_inspect_files(
        &self,
        Parameters(request): Parameters<WorkspaceInspectFilesParams>,
    ) -> Result<Json<WorkspaceInspectFilesResult>, ErrorData> {
        app_json(workspace_inspect_files(request))
    }

    #[tool(
        name = "workspace_inspect_entries",
        description = "List workspace entries without creating a translation claim.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectEntriesParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectEntriesResult>()
    )]
    pub async fn workspace_inspect_entries(
        &self,
        Parameters(request): Parameters<WorkspaceInspectEntriesParams>,
    ) -> Result<Json<WorkspaceInspectEntriesResult>, ErrorData> {
        app_json(workspace_inspect_entries(request))
    }

    #[tool(
        name = "workspace_inspect_entry",
        description = "Read one workspace entry by id without editing the workspace.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectEntryParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectEntry>()
    )]
    pub async fn workspace_inspect_entry(
        &self,
        Parameters(request): Parameters<WorkspaceInspectEntryParams>,
    ) -> Result<Json<WorkspaceInspectEntry>, ErrorData> {
        app_json(workspace_inspect_entry(request))
    }

    #[tool(
        name = "workspace_inspect_diagnostics",
        description = "List workspace diagnostics expanded with entry context for review.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectDiagnosticsParams>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectDiagnosticsResult>()
    )]
    pub async fn workspace_inspect_diagnostics(
        &self,
        Parameters(request): Parameters<WorkspaceInspectDiagnosticsParams>,
    ) -> Result<Json<WorkspaceInspectDiagnosticsResult>, ErrorData> {
        app_json(workspace_inspect_diagnostics(request))
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
        app_json(adapt_import(request).await)
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
        app_json(knowledge_annotate(request))
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
        app_json(knowledge_validate(request))
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
        app_json(knowledge_lookup(request))
    }

    #[tool(
        name = "knowledge_index_rebuild",
        description = "Rebuild the derived knowledge index for a workspace.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeIndexRebuildParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeIndexRebuildResult>()
    )]
    pub async fn knowledge_index_rebuild(
        &self,
        Parameters(request): Parameters<KnowledgeIndexRebuildParams>,
    ) -> Result<Json<KnowledgeIndexRebuildResult>, ErrorData> {
        app_json(knowledge_index_rebuild(request))
    }

    #[tool(
        name = "knowledge_term_upsert",
        description = "Create or replace workspace terminology entries.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermUpsertParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermsEditResult>()
    )]
    pub async fn knowledge_term_upsert(
        &self,
        Parameters(request): Parameters<KnowledgeTermUpsertParams>,
    ) -> Result<Json<KnowledgeTermsEditResult>, ErrorData> {
        app_json(knowledge_term_upsert(request))
    }

    #[tool(
        name = "knowledge_term_delete",
        description = "Delete a workspace terminology entry by id.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermDeleteParams>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermEditResult>()
    )]
    pub async fn knowledge_term_delete(
        &self,
        Parameters(request): Parameters<KnowledgeTermDeleteParams>,
    ) -> Result<Json<KnowledgeTermEditResult>, ErrorData> {
        app_json(knowledge_term_delete(request))
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

fn app_json<T>(result: Result<T, AppError>) -> Result<Json<T>, ErrorData>
where
    T: Serialize + JsonSchema,
{
    Ok(Json(result.map_err(app_error)?))
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
