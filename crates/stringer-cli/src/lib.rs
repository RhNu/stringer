#![forbid(unsafe_code)]

mod app;
mod feedback;
mod help;
mod workspace;

pub use app::{
    AdaptCommand, AdaptFormatArg, AdaptImportCommand, Cli, CliError, Command,
    KnowledgeAnnotateCommand, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeIndexRebuildCommand, KnowledgeLookupCommand, KnowledgeLookupFieldArg,
    KnowledgeLookupSourceArg, KnowledgeTermCommand, KnowledgeTermDeleteCommand,
    KnowledgeTermStatusArg, KnowledgeTermUpsertCommand, KnowledgeValidateCommand, run,
};
pub use feedback::ProgressModeArg;
pub use workspace::{
    InspectDiagnosticSeverityArg, InspectEntryStatusArg, WorkspaceBatchApplyCommand,
    WorkspaceBatchClaimCommand, WorkspaceBatchCommand, WorkspaceBatchCountCommand,
    WorkspaceBatchReleaseCommand, WorkspaceCommand, WorkspaceFinalizeCommand,
    WorkspaceInspectBatchCommand, WorkspaceInspectCommand, WorkspaceInspectDiagnosticsCommand,
    WorkspaceInspectEntriesCommand, WorkspaceInspectEntryCommand, WorkspaceInspectFilesCommand,
    WorkspaceNormalizeCommand, WorkspaceNormalizeEncodingArg, WorkspaceOpenCommand,
    WorkspaceUpgradeCommand,
};
