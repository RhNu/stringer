use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SettingsParam {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_locale: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOpenParams {
    pub root: String,
    pub workspace: String,
    #[serde(default)]
    pub settings: SettingsParam,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOpenResult {
    pub entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceFinalizeParams {
    pub root: String,
    pub workspace: String,
    pub override_root: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceFinalizeResult {
    pub applied_entries: usize,
    pub written_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchCountParams {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchCountResult {
    pub total: usize,
    pub empty: usize,
    pub memory_prefilled: usize,
    pub translated: usize,
    pub claimed: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchClaimParams {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchClaimResult {
    pub batch_id: Option<String>,
    pub entries: Vec<WorkspaceBatchClaimEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchClaimEntry {
    pub id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_meta: Option<Value>,
    pub context: BTreeMap<String, String>,
    pub hints: Vec<Value>,
    pub diagnostics: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchApplyParams {
    pub workspace: String,
    pub batch_id: String,
    pub entries: Vec<WorkspaceBatchApplyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchApplyEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchApplyResult {
    pub applied_entries: usize,
    pub remaining_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReleaseParams {
    pub workspace: String,
    pub batch_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceBatchReleaseResult {
    pub released_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFilesParams {
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFilesResult {
    pub files: Vec<WorkspaceInspectFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectFile {
    pub path: String,
    pub kind: String,
    pub asset_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectEntryStatusParam {
    #[default]
    All,
    Empty,
    Memory,
    Translated,
    Claimed,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntriesParams {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub status: InspectEntryStatusParam,
    #[serde(default = "default_inspect_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntriesResult {
    pub total: usize,
    pub entries: Vec<WorkspaceInspectEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectEntry {
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
pub struct WorkspaceInspectEntryParams {
    pub workspace: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectBatchParams {
    pub workspace: String,
    pub batch_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectBatchResult {
    pub batch_id: String,
    pub entries: Vec<WorkspaceInspectEntry>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InspectDiagnosticSeverityParam {
    #[default]
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnosticsParams {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default)]
    pub severity: InspectDiagnosticSeverityParam,
    #[serde(default = "default_inspect_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnosticsResult {
    pub total: usize,
    pub diagnostics: Vec<WorkspaceInspectDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceInspectDiagnostic {
    pub entry_id: String,
    pub file: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    pub context: BTreeMap<String, String>,
    pub diagnostic: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptFormatParam {
    Eet,
    EetXml,
    EetJson,
    XtSst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdaptImportParams {
    pub format: AdaptFormatParam,
    pub input: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    pub source_locale: String,
    pub target_locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdaptImportResult {
    pub summary: AdaptImportSummary,
    pub action: String,
    pub output: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdaptImportSummary {
    pub total_entries: usize,
    pub written_entries: usize,
    pub skipped_entries: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeAnnotateParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub workspace: String,
    #[serde(default)]
    pub skip_fill_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeValidateParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeLookupParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub text: String,
    #[serde(default)]
    pub kind: KnowledgeKindParam,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrecord: Option<String>,
    #[serde(default)]
    pub settings: SettingsParam,
    #[serde(default)]
    pub regex: bool,
    #[serde(default = "default_lookup_limit")]
    pub limit: usize,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub source: KnowledgeLookupSourceParam,
    #[serde(default)]
    pub field: KnowledgeLookupFieldParam,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeKindParam {
    #[default]
    Plugin,
    Strings,
    Scaleform,
    Pex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupSourceParam {
    #[default]
    All,
    Memory,
    Terms,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupFieldParam {
    #[default]
    Both,
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeIndexRebuildParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub settings: SettingsParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermUpsertParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub terms: Vec<KnowledgeTermParam>,
    #[serde(default)]
    pub rebuild_index: bool,
    #[serde(default)]
    pub settings: SettingsParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermDeleteParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub id: String,
    #[serde(default)]
    pub rebuild_index: bool,
    #[serde(default)]
    pub settings: SettingsParam,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermParam {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub status: KnowledgeTermStatusParam,
    #[serde(default)]
    pub scope: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeTermStatusParam {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeOperationResult {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: usize,
    pub auto_filled: usize,
    pub knowledge_diagnostics: usize,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeLookupResult {
    pub query: String,
    pub mode: String,
    pub total_matches: usize,
    pub results: Vec<KnowledgeLookupMatch>,
    pub diagnostics: Vec<Value>,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeLookupMatch {
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeIndexRebuildResult {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermEditResult {
    pub action: String,
    pub id: String,
    pub path: String,
    pub index_rebuilt: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_summary: Option<KnowledgeIndexRebuildResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermsEditResult {
    pub action: String,
    pub ids: Vec<String>,
    pub count: usize,
    pub path: String,
    pub index_rebuilt: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_summary: Option<KnowledgeIndexRebuildResult>,
}

fn default_lookup_limit() -> usize {
    20
}

fn default_inspect_limit() -> usize {
    50
}
