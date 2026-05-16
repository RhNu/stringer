use std::io;

use camino::Utf8PathBuf;
use stringer_workspace_core::WorkspaceCoreError;
use stringer_workspace_ops::WorkspaceOpsError;
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

    #[error(
        "batch `{batch_id}` was not found; it may already be fully applied, released, or cleared; claim a fresh batch before applying translations"
    )]
    BatchNotFound { batch_id: String },

    #[error("duplicate batch patch id `{id}`")]
    DuplicateBatchPatchId { id: String },

    #[error("batch patch entry `{id}` is missing translation")]
    MissingBatchPatchTranslation { id: String },

    #[error(
        "translation id `{id}` is not claimed by batch `{batch_id}`; it is not in the remaining batch entries; re-read the batch from offset 0 before retrying"
    )]
    BatchEntryNotClaimed { batch_id: String, id: String },

    #[error("duplicate output logical path `{path}`")]
    DuplicateOutputPath { path: String },

    #[error("invalid output logical path `{path}`: {message}")]
    InvalidLogicalPath { path: String, message: String },

    #[error("invalid output root `{root}`: {message}")]
    InvalidOutputRoot { root: Utf8PathBuf, message: String },

    #[error(transparent)]
    Reader(#[from] stringer_reader::ReaderError),

    #[error(transparent)]
    Plugin(#[from] stringer_plugin::PluginError),

    #[error(transparent)]
    Pex(#[from] stringer_pex::PexError),

    #[error(transparent)]
    Scaleform(#[from] stringer_scaleform::ScaleformError),

    #[error(transparent)]
    Bundle(#[from] stringer_core::StringerCoreError),
}

impl From<WorkspaceCoreError> for WorkspaceError {
    fn from(source: WorkspaceCoreError) -> Self {
        match source {
            WorkspaceCoreError::ReadFile { path, source } => Self::ReadFile { path, source },
            WorkspaceCoreError::WriteFile { path, source } => Self::WriteFile { path, source },
            WorkspaceCoreError::CurrentDirectory { source } => Self::CurrentDirectory { source },
            WorkspaceCoreError::ConfigToml { path, source } => Self::ConfigToml { path, source },
            WorkspaceCoreError::Toml { path, source } => Self::Toml { path, source },
            WorkspaceCoreError::MissingSetting { name } => Self::MissingSetting { name },
            WorkspaceCoreError::InvalidSetting { name, value } => {
                Self::InvalidSetting { name, value }
            }
            WorkspaceCoreError::JsonLine { path, line, source } => {
                Self::JsonLine { path, line, source }
            }
            WorkspaceCoreError::Json { path, source } => Self::Json { path, source },
            WorkspaceCoreError::UnsupportedTranslationSchema { path, version } => {
                Self::UnsupportedTranslationSchema { path, version }
            }
            WorkspaceCoreError::LegacyTranslationWorkspace { path } => {
                Self::LegacyTranslationWorkspace { path }
            }
            WorkspaceCoreError::WorkspaceLocked { path } => Self::WorkspaceLocked { path },
            WorkspaceCoreError::InvalidTranslationPackagePath { path, message } => {
                Self::InvalidTranslationPackagePath { path, message }
            }
            WorkspaceCoreError::InvalidLogicalPath { path, message } => {
                Self::InvalidLogicalPath { path, message }
            }
            WorkspaceCoreError::DuplicateTranslationId { path, id } => {
                Self::DuplicateTranslationId { path, id }
            }
            WorkspaceCoreError::BatchNotFound { batch_id } => Self::BatchNotFound { batch_id },
        }
    }
}

impl From<WorkspaceOpsError> for WorkspaceError {
    fn from(source: WorkspaceOpsError) -> Self {
        match source {
            WorkspaceOpsError::Core(source) => source.into(),
            WorkspaceOpsError::UnknownTranslationId { id } => Self::UnknownTranslationId { id },
            WorkspaceOpsError::DuplicateBatchPatchId { id } => Self::DuplicateBatchPatchId { id },
            WorkspaceOpsError::MissingBatchPatchTranslation { id } => {
                Self::MissingBatchPatchTranslation { id }
            }
            WorkspaceOpsError::BatchEntryNotClaimed { batch_id, id } => {
                Self::BatchEntryNotClaimed { batch_id, id }
            }
        }
    }
}
