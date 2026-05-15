use camino::Utf8PathBuf;
use stringer_workspace_core::WorkspaceCoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KnowledgeError {
    #[error(transparent)]
    Core(#[from] WorkspaceCoreError),

    #[error("failed to parse knowledge terms TOML `{path}`: {source}")]
    KnowledgeTermsToml {
        path: Utf8PathBuf,
        #[source]
        source: Box<toml_edit::TomlError>,
    },

    #[error("invalid knowledge terms TOML `{path}`: {message}")]
    InvalidKnowledgeTermsToml { path: Utf8PathBuf, message: String },

    #[error("knowledge term `{id}` was not found in `{path}`")]
    KnowledgeTermNotFound { path: Utf8PathBuf, id: String },

    #[error("invalid knowledge term scope key `{key}` for term `{id}`")]
    InvalidKnowledgeTermScope { id: String, key: String },

    #[error("invalid knowledge term file `{path}`: {message}")]
    InvalidKnowledgeTermFile { path: Utf8PathBuf, message: String },

    #[error("invalid lookup regex `{pattern}`: {source}")]
    InvalidLookupRegex {
        pattern: String,
        #[source]
        source: regex::Error,
    },

    #[error("failed to process SQLite index `{path}`: {source}")]
    Sqlite {
        path: Utf8PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error(transparent)]
    Pipeline(Box<stringer_pipeline::PipelineError>),
}

impl From<stringer_pipeline::PipelineError> for KnowledgeError {
    fn from(source: stringer_pipeline::PipelineError) -> Self {
        Self::Pipeline(Box::new(source))
    }
}
