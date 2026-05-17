use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOpenRequest {
    pub source_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub settings: SettingsInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceOpenResponse {
    pub entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceFinalizeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceFinalizeResponse {
    pub applied_entries: usize,
    pub written_files: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceNormalizeEncodingInput {
    #[default]
    Auto,
    #[serde(rename = "utf-8")]
    Utf8,
    Cp936,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceNormalizeResponse {
    pub scanned_entries: usize,
    pub changed_entries: usize,
    pub total_replacements: usize,
    pub skipped_claimed: usize,
    pub skipped_placeholder_risk: usize,
    pub warnings: Vec<WorkspaceNormalizeWarningResponse>,
    pub changes: Vec<WorkspaceNormalizeChangeResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

fn default_normalize_limit() -> usize {
    50
}

fn is_false(value: &bool) -> bool {
    !*value
}
