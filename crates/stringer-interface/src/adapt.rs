use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AdaptFormatInput {
    Eet,
    EetXml,
    EetJson,
    XtSst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AdaptImportResponse {
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
