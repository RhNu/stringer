use std::io;

use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceCoreError {
    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to write `{path}`: {source}")]
    WriteFile {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to resolve current directory: {source}")]
    CurrentDirectory {
        #[source]
        source: io::Error,
    },

    #[error("failed to parse TOML config `{path}`: {source}")]
    ConfigToml {
        path: Utf8PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(transparent)]
    ExtractionFilter(#[from] stringer_extraction_filter::ExtractionFilterError),

    #[error("failed to write TOML `{path}`: {source}")]
    Toml {
        path: Utf8PathBuf,
        #[source]
        source: toml::ser::Error,
    },

    #[error("missing workspace setting `{name}`")]
    MissingSetting { name: &'static str },

    #[error("invalid workspace setting `{name}` value `{value}`")]
    InvalidSetting { name: &'static str, value: String },

    #[error("failed to parse JSONL `{path}` line {line}: {source}")]
    JsonLine {
        path: Utf8PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to process JSON `{path}`: {source}")]
    Json {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("unsupported translation package schema version {version} in `{path}`")]
    UnsupportedTranslationSchema { path: Utf8PathBuf, version: u32 },

    #[error("unsupported batch format version {version} in `{path}`")]
    UnsupportedBatchFormat { path: Utf8PathBuf, version: u32 },

    #[error(
        "legacy translation workspace `{path}` uses manifest.json; recreate it with workspace open"
    )]
    LegacyTranslationWorkspace { path: Utf8PathBuf },

    #[error("workspace is locked by `{path}`")]
    WorkspaceLocked { path: Utf8PathBuf },

    #[error("invalid translation package path `{path}`: {message}")]
    InvalidTranslationPackagePath { path: String, message: String },

    #[error("invalid output logical path `{path}`: {message}")]
    InvalidLogicalPath { path: String, message: String },

    #[error("duplicate translation id `{id}` in `{path}`")]
    DuplicateTranslationId { path: Utf8PathBuf, id: String },

    #[error(
        "batch `{batch_id}` was not found; it may already be completed, released, or cleared; claim a fresh batch before submitting translations"
    )]
    BatchNotFound { batch_id: String },
}
