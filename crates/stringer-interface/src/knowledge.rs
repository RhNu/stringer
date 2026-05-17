use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeAnnotateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub skip_fill_memory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeValidateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeKindInput {
    #[default]
    Plugin,
    Strings,
    Scaleform,
    Pex,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupSourceInput {
    #[default]
    All,
    Memory,
    Terms,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeLookupFieldInput {
    #[default]
    Both,
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeIndexRebuildRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermUpsertRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub terms: Vec<KnowledgeTermInput>,
    #[serde(default)]
    pub rebuild_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermDeleteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub id: String,
    #[serde(default)]
    pub rebuild_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeTermStatusInput {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeOperationResponse {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: usize,
    pub auto_filled: usize,
    pub knowledge_diagnostics: usize,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeLookupResponse {
    pub query: String,
    pub mode: String,
    pub total_matches: usize,
    pub results: Vec<KnowledgeLookupResultResponse>,
    pub diagnostics: Vec<Value>,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgeTermEditResponse {
    pub action: String,
    pub id: String,
    pub path: String,
    pub index_rebuilt: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_summary: Option<KnowledgeIndexRebuildResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
