#![forbid(unsafe_code)]

mod app;
mod cli_settings;
mod help;
mod workspace;

pub use app::{
    AdaptCommand, AdaptFormatArg, AdaptImportCommand, Cli, CliError, Command,
    KnowledgeAnnotateCommand, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeIndexRebuildCommand, KnowledgeLookupCommand, KnowledgeLookupFieldArg,
    KnowledgeLookupSourceArg, KnowledgeValidateCommand, run,
};
pub use workspace::{
    WorkspaceBatchApplyCommand, WorkspaceBatchClaimCommand, WorkspaceBatchCommand,
    WorkspaceBatchCountCommand, WorkspaceBatchReleaseCommand, WorkspaceCommand,
    WorkspaceFinalizeCommand, WorkspaceOpenCommand, WorkspaceUpgradeCommand,
};
