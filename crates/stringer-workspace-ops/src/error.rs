use stringer_workspace_core::WorkspaceCoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceOpsError {
    #[error(transparent)]
    Core(#[from] WorkspaceCoreError),

    #[error("translation id `{id}` does not match any exported entry")]
    UnknownTranslationId { id: String },

    #[error("duplicate batch patch id `{id}`")]
    DuplicateBatchPatchId { id: String },

    #[error("batch patch entry `{id}` is missing translation or skip=true")]
    MissingBatchPatchTranslation { id: String },

    #[error(
        "translation id `{id}` is not claimed by batch `{batch_id}`; it is not in the remaining batch entries; re-read the batch from offset 0 before retrying"
    )]
    BatchEntryNotClaimed { batch_id: String, id: String },

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
