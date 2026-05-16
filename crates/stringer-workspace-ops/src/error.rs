use stringer_workspace_core::WorkspaceCoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceOpsError {
    #[error(transparent)]
    Core(#[from] WorkspaceCoreError),

    #[error("translation id `{id}` does not match any exported entry")]
    UnknownTranslationId { id: String },

    #[error(
        "batch `{batch_id}` revision conflict: expected {expected}, current {current}; re-read the batch before retrying"
    )]
    BatchRevisionConflict {
        batch_id: String,
        expected: u64,
        current: u64,
    },

    #[error("batch `{batch_id}` detail requires at least one key")]
    BatchDetailKeysRequired { batch_id: String },

    #[error("failed to decode normalization rules `{path}` as {encoding}")]
    NormalizeRuleDecode {
        path: camino::Utf8PathBuf,
        encoding: &'static str,
    },

    #[error("failed to parse normalization rules `{path}` line {line}: {message}")]
    NormalizeRuleParse {
        path: camino::Utf8PathBuf,
        line: usize,
        message: String,
    },
}
