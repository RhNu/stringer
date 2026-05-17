use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFilesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFilesResponse {
    pub files: Vec<WorkspaceInspectFileResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFileResponse {
    pub path: String,
    pub kind: String,
    pub asset_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectEntryStatusInput {
    #[default]
    All,
    Empty,
    Memory,
    Translated,
    Skipped,
    Claimed,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntriesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub status: InspectEntryStatusInput,
    #[serde(default = "default_inspect_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntriesResponse {
    pub total: usize,
    pub entries: Vec<WorkspaceInspectEntrySummaryResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntrySummaryResponse {
    pub file: String,
    pub id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub context_label: String,
    pub hint_count: usize,
    pub diagnostic_count: usize,
    pub diagnostic_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntryResponse {
    pub file: String,
    pub id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_meta: Option<Value>,
    pub context: BTreeMap<String, String>,
    pub hints: Vec<Value>,
    pub diagnostics: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectDiagnosticSeverityInput {
    #[default]
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnosticsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub severity: InspectDiagnosticSeverityInput,
    #[serde(default = "default_inspect_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnosticsResponse {
    pub total: usize,
    pub diagnostics: Vec<WorkspaceInspectDiagnosticResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnosticResponse {
    pub entry_id: String,
    pub file: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_translation: Option<String>,
    pub context_label: String,
    pub code: String,
    pub severity: String,
    pub message: String,
}

fn default_inspect_limit() -> usize {
    50
}
