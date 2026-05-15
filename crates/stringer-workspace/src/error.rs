use std::io;

use camino::Utf8PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
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

    #[error("invalid lookup regex `{pattern}`: {source}")]
    InvalidLookupRegex {
        pattern: String,
        #[source]
        source: regex::Error,
    },

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

    #[error("failed to process SQLite index `{path}`: {source}")]
    Sqlite {
        path: Utf8PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("unsupported translation package schema version {version} in `{path}`")]
    UnsupportedTranslationSchema { path: Utf8PathBuf, version: u32 },

    #[error(
        "legacy translation workspace `{path}` uses manifest.json; recreate or upgrade it to workspace.json"
    )]
    LegacyTranslationWorkspace { path: Utf8PathBuf },

    #[error("workspace is locked by `{path}`")]
    WorkspaceLocked { path: Utf8PathBuf },

    #[error("invalid translation package path `{path}`: {message}")]
    InvalidTranslationPackagePath { path: String, message: String },

    #[error("duplicate translation id `{id}` in `{path}`")]
    DuplicateTranslationId { path: Utf8PathBuf, id: String },

    #[error("translation id `{id}` does not match any exported entry")]
    UnknownTranslationId { id: String },

    #[error("batch `{batch_id}` was not found")]
    BatchNotFound { batch_id: String },

    #[error("duplicate batch patch id `{id}`")]
    DuplicateBatchPatchId { id: String },

    #[error("batch patch entry `{id}` is missing translation")]
    MissingBatchPatchTranslation { id: String },

    #[error("translation id `{id}` is not claimed by batch `{batch_id}`")]
    BatchEntryNotClaimed { batch_id: String, id: String },

    #[error("duplicate output logical path `{path}`")]
    DuplicateOutputPath { path: String },

    #[error("invalid override logical path `{path}`: {message}")]
    InvalidLogicalPath { path: String, message: String },

    #[error("invalid override root `{root}`: {message}")]
    InvalidOverrideRoot { root: Utf8PathBuf, message: String },

    #[error(transparent)]
    Reader(#[from] stringer_reader::ReaderError),

    #[error(transparent)]
    Plugin(#[from] stringer_plugin::PluginError),

    #[error(transparent)]
    Pex(#[from] stringer_pex::PexError),

    #[error(transparent)]
    Pipeline(Box<stringer_pipeline::PipelineError>),

    #[error(transparent)]
    Scaleform(#[from] stringer_scaleform::ScaleformError),

    #[error(transparent)]
    Bundle(#[from] stringer_core::StringerCoreError),
}

impl From<stringer_pipeline::PipelineError> for WorkspaceError {
    fn from(source: stringer_pipeline::PipelineError) -> Self {
        Self::Pipeline(Box::new(source))
    }
}
