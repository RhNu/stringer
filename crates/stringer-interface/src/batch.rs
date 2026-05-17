use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchCountRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchCountResponse {
    pub total: usize,
    pub claimable: usize,
    pub empty: usize,
    pub memory_prefilled: usize,
    pub translated: usize,
    pub skipped: usize,
    pub claimed: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchClaimRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchClaimResponse {
    pub batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub claimed_entries: usize,
    pub remaining_claimable: usize,
    pub scope: WorkspaceBatchScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReleaseRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReleaseResponse {
    pub released_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_batch_read_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchDetailRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReadResponse {
    pub batch_id: String,
    pub revision: u64,
    pub total_entries: usize,
    pub open_entries: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    pub entries: Vec<WorkspaceBatchReadEntryResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReadEntryResponse {
    pub key: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub context_label: String,
    pub hint_count: usize,
    pub diagnostic_count: usize,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchDetailResponse {
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<WorkspaceBatchDetailEntryResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchDetailEntryResponse {
    pub key: String,
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
pub struct WorkspaceBatchSubmitRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<WorkspaceBatchSubmitEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchSubmitEntry {
    pub key: String,
    pub action: WorkspaceBatchSubmitActionInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchSubmitActionInput {
    Translate,
    Skip,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchSubmitResponse {
    pub batch_id: String,
    pub revision: u64,
    pub applied_entries: usize,
    pub ignored_entries: usize,
    pub rejected_entries: usize,
    pub remaining_entries: usize,
    pub next_read_offset: usize,
    pub results: Vec<WorkspaceBatchSubmitEntryResultResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchSubmitEntryResultResponse {
    pub key: String,
    pub status: WorkspaceBatchSubmitStatusResponse,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchSubmitStatusResponse {
    Applied,
    Ignored,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchExportFormatInput {
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchExportRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(default = "default_batch_export_format")]
    pub format: WorkspaceBatchExportFormatInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchExportResponse {
    pub path: String,
    pub format: WorkspaceBatchExportFormatInput,
    pub entries: usize,
}

fn default_batch_read_limit() -> usize {
    10
}

fn default_batch_export_format() -> WorkspaceBatchExportFormatInput {
    WorkspaceBatchExportFormatInput::Json
}
