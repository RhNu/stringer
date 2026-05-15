use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
pub struct WorkspaceOpenRequest {
    pub root: String,
    pub workspace: String,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceOpenResponse {
    pub entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFinalizeRequest {
    pub root: String,
    pub workspace: String,
    pub override_root: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFinalizeResponse {
    pub applied_entries: usize,
    pub written_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchCountRequest {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchCountResponse {
    pub total: usize,
    pub empty: usize,
    pub memory_prefilled: usize,
    pub translated: usize,
    pub claimed: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchClaimRequest {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceBatchClaimResponse {
    pub batch_id: Option<String>,
    pub entries: Vec<WorkspaceBatchClaimEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchApplyRequest {
    pub workspace: String,
    pub batch_id: String,
    pub entries: Vec<WorkspaceBatchApplyEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchApplyEntry {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchApplyResponse {
    pub applied_entries: usize,
    pub remaining_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchReleaseRequest {
    pub workspace: String,
    pub batch_id: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBatchReleaseResponse {
    pub released_entries: usize,
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
pub struct KnowledgeAnnotateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub workspace: String,
    #[serde(default)]
    pub skip_fill_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeValidateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub workspace: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeLookupRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub text: String,
    #[serde(default)]
    pub kind: KnowledgeKindInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subrecord: Option<String>,
    #[serde(default)]
    pub settings: SettingsInput,
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
pub struct KnowledgeIndexRebuildRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTermUpsertRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub term: KnowledgeTermInput,
    #[serde(default)]
    pub rebuild_index: bool,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTermDeleteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub id: String,
    #[serde(default)]
    pub rebuild_index: bool,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeIndexRebuildResponse {
    pub files: usize,
    pub terms: usize,
    pub memory: usize,
    pub rules: usize,
    pub diagnostics: usize,
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

fn default_lookup_limit() -> usize {
    20
}
