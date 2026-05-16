use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceOpenRequest {
    pub source_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOpenResponse {
    pub entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceFinalizeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFinalizeResponse {
    pub applied_entries: usize,
    pub written_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchCountRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchClaimRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceBatchClaimResponse {
    pub batch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    pub claimed_entries: usize,
    pub remaining_claimable: usize,
    pub scope: WorkspaceBatchScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchApplyRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    pub entries: Vec<WorkspaceBatchApplyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchApplyEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default)]
    pub skip: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchApplyResponse {
    pub applied_entries: usize,
    pub remaining_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchReleaseRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchReleaseResponse {
    pub released_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchReadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_batch_read_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchDetailRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceBatchDetailResponse {
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<WorkspaceBatchDetailEntryResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchSubmitRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    pub revision: u64,
    pub entries: Vec<WorkspaceBatchSubmitEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchSubmitEntry {
    pub key: String,
    pub action: WorkspaceBatchSubmitActionInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchSubmitActionInput {
    Translate,
    Skip,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchSubmitEntryResultResponse {
    pub key: String,
    pub status: WorkspaceBatchSubmitStatusResponse,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchSubmitStatusResponse {
    Applied,
    Ignored,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceBatchExportFormatInput {
    Json,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceBatchExportRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(default = "default_batch_export_format")]
    pub format: WorkspaceBatchExportFormatInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchExportResponse {
    pub path: String,
    pub format: WorkspaceBatchExportFormatInput,
    pub entries: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceNormalizeEncodingInput {
    #[default]
    Auto,
    #[serde(rename = "utf-8")]
    Utf8,
    Cp936,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceNormalizeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub rules: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub apply: bool,
    #[serde(default)]
    pub encoding: WorkspaceNormalizeEncodingInput,
    #[serde(default = "default_normalize_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNormalizeResponse {
    pub scanned_entries: usize,
    pub changed_entries: usize,
    pub total_replacements: usize,
    pub skipped_claimed: usize,
    pub skipped_placeholder_risk: usize,
    pub warnings: Vec<WorkspaceNormalizeWarningResponse>,
    pub changes: Vec<WorkspaceNormalizeChangeResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNormalizeWarningResponse {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNormalizeChangeResponse {
    pub file: String,
    pub id: String,
    pub source: String,
    pub before: String,
    pub after: String,
    pub replacements: usize,
    pub rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub skipped_placeholder_risk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspectFilesRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInspectFilesResponse {
    pub files: Vec<WorkspaceInspectFileResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceInspectFileResponse {
    pub path: String,
    pub kind: String,
    pub asset_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceInspectEntriesResponse {
    pub total: usize,
    pub entries: Vec<WorkspaceInspectEntrySummaryResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspectEntryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspectBatchRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub batch_id: String,
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_batch_inspect_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceInspectBatchResponse {
    pub batch_id: String,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    pub entries: Vec<WorkspaceInspectEntryResponse>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InspectDiagnosticSeverityInput {
    #[default]
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceInspectDiagnosticsResponse {
    pub total: usize,
    pub diagnostics: Vec<WorkspaceInspectDiagnosticResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptFormatInput {
    Eet,
    EetXml,
    EetJson,
    XtSst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptImportRequest {
    pub format: AdaptFormatInput,
    pub input: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    pub source_locale: String,
    pub target_locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptImportResponse {
    pub summary: AdaptImportSummary,
    pub action: String,
    pub output: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptImportSummary {
    pub total_entries: usize,
    pub written_entries: usize,
    pub skipped_entries: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeAnnotateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub skip_fill_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeValidateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeLookupRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    pub text: String,
    #[serde(default)]
    pub kind: KnowledgeKindInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrecord: Option<String>,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_lookup_limit")]
    pub limit: usize,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub source: KnowledgeLookupSourceInput,
    #[serde(default)]
    pub field: KnowledgeLookupFieldInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeKindInput {
    #[default]
    Plugin,
    Strings,
    Scaleform,
    Pex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupSourceInput {
    #[default]
    All,
    Memory,
    Terms,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupFieldInput {
    #[default]
    Both,
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeIndexRebuildRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeTermUpsertRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub terms: Vec<KnowledgeTermInput>,
    #[serde(default)]
    pub rebuild_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeTermDeleteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub id: String,
    #[serde(default)]
    pub rebuild_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeTermInput {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub status: KnowledgeTermStatusInput,
    #[serde(default)]
    pub scope: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeTermStatusInput {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeOperationResponse {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: usize,
    pub auto_filled: usize,
    pub knowledge_diagnostics: usize,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeLookupResponse {
    pub query: String,
    pub mode: String,
    pub total_matches: usize,
    pub results: Vec<KnowledgeLookupResultResponse>,
    pub diagnostics: Vec<Value>,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeLookupResultResponse {
    pub kind: String,
    pub id: String,
    pub layer: String,
    pub source: String,
    pub target: String,
    pub match_field: String,
    pub match_kind: String,
    pub score: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeIndexRebuildResponse {
    pub files: usize,
    pub terms: usize,
    pub memory: usize,
    pub rules: usize,
    pub diagnostics: usize,
    pub indexed_items: usize,
    pub fts_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebuild_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTermEditResponse {
    pub action: String,
    pub id: String,
    pub path: String,
    pub index_rebuilt: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_summary: Option<KnowledgeIndexRebuildResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTermsEditResponse {
    pub action: String,
    pub ids: Vec<String>,
    pub count: usize,
    pub path: String,
    pub index_rebuilt: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_summary: Option<KnowledgeIndexRebuildResponse>,
}

fn default_lookup_limit() -> usize {
    20
}

fn default_batch_read_limit() -> usize {
    10
}

fn default_batch_export_format() -> WorkspaceBatchExportFormatInput {
    WorkspaceBatchExportFormatInput::Json
}

fn default_normalize_limit() -> usize {
    50
}

fn default_batch_inspect_limit() -> usize {
    10
}

fn default_inspect_limit() -> usize {
    50
}

fn is_false(value: &bool) -> bool {
    !*value
}
