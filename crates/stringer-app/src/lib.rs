#![forbid(unsafe_code)]

mod adapt;
mod context;
mod error;
mod knowledge;
mod paths;
mod settings;
mod workspace;

pub use adapt::adapt_import;
pub use context::StringerApp;
pub use error::{AppError, AppErrorPayload};
pub use knowledge::{
    knowledge_annotate, knowledge_annotate_with_progress, knowledge_index_rebuild,
    knowledge_index_rebuild_with_progress, knowledge_lookup, knowledge_term_delete,
    knowledge_term_upsert, knowledge_validate, knowledge_validate_with_progress,
    parse_knowledge_kind,
};
pub use stringer_knowledge::{KnowledgeOperation, KnowledgeProgressEvent, KnowledgeProgressPhase};
pub use workspace::{
    workspace_batch_claim, workspace_batch_count, workspace_batch_detail, workspace_batch_export,
    workspace_batch_read, workspace_batch_release, workspace_batch_submit, workspace_finalize,
    workspace_inspect_diagnostics, workspace_inspect_entries, workspace_inspect_entry,
    workspace_inspect_files, workspace_normalize, workspace_open,
};
