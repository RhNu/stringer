use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("failed to parse terms TOML `{path}`: {source}")]
    TermsToml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to parse replacement rules TOML `{path}`: {source}")]
    RulesToml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to parse memory JSONL `{path}` line {line}: {source}")]
    MemoryJsonLine {
        path: String,
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("duplicate knowledge id `{id}` in `{path}`")]
    DuplicateKnowledgeId { path: String, id: String },
}
