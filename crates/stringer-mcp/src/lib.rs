#![forbid(unsafe_code)]

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
use stringer_interface::*;

#[derive(Debug, Clone, Default)]
pub struct StringerMcp;

#[tool_router(server_handler)]
impl StringerMcp {
    #[tool(
        name = "workspace_open",
        description = "Open a translation workspace from a read-only source root.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceOpenRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceOpenResponse>()
    )]
    pub async fn workspace_open(
        &self,
        Parameters(request): Parameters<WorkspaceOpenRequest>,
    ) -> Result<Json<WorkspaceOpenResponse>, ErrorData> {
        app_json(workspace_open(request).await)
    }

    #[tool(
        name = "workspace_finalize",
        description = "Finalize a translation workspace into an output directory.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceFinalizeRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceFinalizeResponse>()
    )]
    pub async fn workspace_finalize(
        &self,
        Parameters(request): Parameters<WorkspaceFinalizeRequest>,
    ) -> Result<Json<WorkspaceFinalizeResponse>, ErrorData> {
        app_json(workspace_finalize(request).await)
    }

    #[tool(
        name = "workspace_batch_count",
        description = "Count translation rows, claims, and diagnostics in a workspace.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchCountRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchCountResponse>()
    )]
    pub async fn workspace_batch_count(
        &self,
        Parameters(request): Parameters<WorkspaceBatchCountRequest>,
    ) -> Result<Json<WorkspaceBatchCountResponse>, ErrorData> {
        app_json(workspace_batch_count(request))
    }

    #[tool(
        name = "workspace_batch_claim",
        description = "Claim eligible translation rows for agent work.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchClaimRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchClaimResponse>()
    )]
    pub async fn workspace_batch_claim(
        &self,
        Parameters(request): Parameters<WorkspaceBatchClaimRequest>,
    ) -> Result<Json<WorkspaceBatchClaimResponse>, ErrorData> {
        app_json(workspace_batch_claim(request))
    }

    #[tool(
        name = "workspace_batch_release",
        description = "Release a claimed batch without submitting translations.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchReleaseRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchReleaseResponse>()
    )]
    pub async fn workspace_batch_release(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReleaseRequest>,
    ) -> Result<Json<WorkspaceBatchReleaseResponse>, ErrorData> {
        app_json(workspace_batch_release(request))
    }

    #[tool(
        name = "workspace_batch_read",
        description = "Read compact entries from a claimed batch.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchReadRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchReadResponse>()
    )]
    pub async fn workspace_batch_read(
        &self,
        Parameters(request): Parameters<WorkspaceBatchReadRequest>,
    ) -> Result<Json<WorkspaceBatchReadResponse>, ErrorData> {
        app_json(workspace_batch_read(request))
    }

    #[tool(
        name = "workspace_batch_detail",
        description = "Read full detail for one or more claimed batch keys.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchDetailRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchDetailResponse>()
    )]
    pub async fn workspace_batch_detail(
        &self,
        Parameters(request): Parameters<WorkspaceBatchDetailRequest>,
    ) -> Result<Json<WorkspaceBatchDetailResponse>, ErrorData> {
        app_json(workspace_batch_detail(request))
    }

    #[tool(
        name = "workspace_batch_submit",
        description = "Submit translate, skip, or pending actions inline, or from an exported patch.json/patch.csv input file.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchSubmitRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchSubmitResponse>()
    )]
    pub async fn workspace_batch_submit(
        &self,
        Parameters(request): Parameters<WorkspaceBatchSubmitRequest>,
    ) -> Result<Json<WorkspaceBatchSubmitResponse>, ErrorData> {
        app_json(workspace_batch_submit(request))
    }

    #[tool(
        name = "workspace_batch_export",
        description = "Export a claimed batch to an editable JSON or CSV submission file.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceBatchExportRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceBatchExportResponse>()
    )]
    pub async fn workspace_batch_export(
        &self,
        Parameters(request): Parameters<WorkspaceBatchExportRequest>,
    ) -> Result<Json<WorkspaceBatchExportResponse>, ErrorData> {
        app_json(workspace_batch_export(request))
    }

    #[tool(
        name = "workspace_normalize",
        description = "Normalize existing translations with xTranslator Search/Replace rules.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceNormalizeRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceNormalizeResponse>()
    )]
    pub async fn workspace_normalize(
        &self,
        Parameters(request): Parameters<WorkspaceNormalizeRequest>,
    ) -> Result<Json<WorkspaceNormalizeResponse>, ErrorData> {
        app_json(workspace_normalize(request))
    }

    #[tool(
        name = "workspace_inspect_files",
        description = "List workspace entry files without reading raw workspace files directly.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectFilesRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectFilesResponse>()
    )]
    pub async fn workspace_inspect_files(
        &self,
        Parameters(request): Parameters<WorkspaceInspectFilesRequest>,
    ) -> Result<Json<WorkspaceInspectFilesResponse>, ErrorData> {
        app_json(workspace_inspect_files(request))
    }

    #[tool(
        name = "workspace_inspect_entries",
        description = "List workspace entries without creating a translation claim.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectEntriesRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectEntriesResponse>()
    )]
    pub async fn workspace_inspect_entries(
        &self,
        Parameters(request): Parameters<WorkspaceInspectEntriesRequest>,
    ) -> Result<Json<WorkspaceInspectEntriesResponse>, ErrorData> {
        app_json(workspace_inspect_entries(request))
    }

    #[tool(
        name = "workspace_inspect_entry",
        description = "Read one workspace entry by id without editing the workspace.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectEntryRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectEntryResponse>()
    )]
    pub async fn workspace_inspect_entry(
        &self,
        Parameters(request): Parameters<WorkspaceInspectEntryRequest>,
    ) -> Result<Json<WorkspaceInspectEntryResponse>, ErrorData> {
        app_json(workspace_inspect_entry(request))
    }

    #[tool(
        name = "workspace_inspect_diagnostics",
        description = "List workspace diagnostics expanded with entry context for review.",
        input_schema = compatible_schema_for_type::<Parameters<WorkspaceInspectDiagnosticsRequest>>(),
        output_schema = compatible_output_schema_for_type::<WorkspaceInspectDiagnosticsResponse>()
    )]
    pub async fn workspace_inspect_diagnostics(
        &self,
        Parameters(request): Parameters<WorkspaceInspectDiagnosticsRequest>,
    ) -> Result<Json<WorkspaceInspectDiagnosticsResponse>, ErrorData> {
        app_json(workspace_inspect_diagnostics(request))
    }

    #[tool(
        name = "adapt_import",
        description = "Import an external translation resource as memory JSONL.",
        input_schema = compatible_schema_for_type::<Parameters<AdaptImportRequest>>(),
        output_schema = compatible_output_schema_for_type::<AdaptImportResponse>()
    )]
    pub async fn adapt_import(
        &self,
        Parameters(request): Parameters<AdaptImportRequest>,
    ) -> Result<Json<AdaptImportResponse>, ErrorData> {
        app_json(adapt_import(request).await)
    }

    #[tool(
        name = "knowledge_annotate",
        description = "Annotate workspace rows with terminology, memory, and diagnostics.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeAnnotateRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeOperationResponse>()
    )]
    pub async fn knowledge_annotate(
        &self,
        Parameters(request): Parameters<KnowledgeAnnotateRequest>,
    ) -> Result<Json<KnowledgeOperationResponse>, ErrorData> {
        app_json(knowledge_annotate(request))
    }

    #[tool(
        name = "knowledge_validate",
        description = "Validate workspace translations and rewrite diagnostics.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeValidateRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeOperationResponse>()
    )]
    pub async fn knowledge_validate(
        &self,
        Parameters(request): Parameters<KnowledgeValidateRequest>,
    ) -> Result<Json<KnowledgeOperationResponse>, ErrorData> {
        app_json(knowledge_validate(request))
    }

    #[tool(
        name = "knowledge_lookup",
        description = "Search terminology and translation memory.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeLookupRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeLookupResponse>()
    )]
    pub async fn knowledge_lookup(
        &self,
        Parameters(request): Parameters<KnowledgeLookupRequest>,
    ) -> Result<Json<KnowledgeLookupResponse>, ErrorData> {
        app_json(knowledge_lookup(request))
    }

    #[tool(
        name = "knowledge_index_rebuild",
        description = "Rebuild the derived knowledge index for a workspace.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeIndexRebuildRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeIndexRebuildResponse>()
    )]
    pub async fn knowledge_index_rebuild(
        &self,
        Parameters(request): Parameters<KnowledgeIndexRebuildRequest>,
    ) -> Result<Json<KnowledgeIndexRebuildResponse>, ErrorData> {
        app_json(knowledge_index_rebuild(request))
    }

    #[tool(
        name = "knowledge_term_upsert",
        description = "Create or replace workspace terminology entries.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermUpsertRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermsEditResponse>()
    )]
    pub async fn knowledge_term_upsert(
        &self,
        Parameters(request): Parameters<KnowledgeTermUpsertRequest>,
    ) -> Result<Json<KnowledgeTermsEditResponse>, ErrorData> {
        app_json(knowledge_term_upsert(request))
    }

    #[tool(
        name = "knowledge_term_delete",
        description = "Delete a workspace terminology entry by id.",
        input_schema = compatible_schema_for_type::<Parameters<KnowledgeTermDeleteRequest>>(),
        output_schema = compatible_output_schema_for_type::<KnowledgeTermEditResponse>()
    )]
    pub async fn knowledge_term_delete(
        &self,
        Parameters(request): Parameters<KnowledgeTermDeleteRequest>,
    ) -> Result<Json<KnowledgeTermEditResponse>, ErrorData> {
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
    let payload = error.payload();
    ErrorData::internal_error(payload.message.clone(), Some(json!(payload)))
}
