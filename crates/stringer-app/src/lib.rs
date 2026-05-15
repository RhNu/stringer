#![forbid(unsafe_code)]

mod adapt;
mod dto;
mod error;
mod knowledge;
mod paths;
mod settings;
mod workspace;

pub use adapt::adapt_import;
pub use dto::*;
pub use error::AppError;
pub use knowledge::{
    knowledge_annotate, knowledge_index_rebuild, knowledge_lookup, knowledge_term_delete,
    knowledge_term_upsert, knowledge_validate, parse_knowledge_kind,
};
pub use workspace::{
    workspace_batch_apply, workspace_batch_claim, workspace_batch_count, workspace_batch_release,
    workspace_finalize, workspace_open, workspace_upgrade_unsupported,
};
