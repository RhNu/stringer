pub use stringer_workspace_ops::{
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, WorkspaceInspectBatch,
    WorkspaceInspectDiagnostic, WorkspaceInspectDiagnostics, WorkspaceInspectEntries,
    WorkspaceInspectEntry, WorkspaceInspectFiles,
};

use crate::WorkspaceError;

pub fn inspect_workspace_files(
    options: InspectWorkspaceFilesOptions,
) -> Result<WorkspaceInspectFiles, WorkspaceError> {
    stringer_workspace_ops::inspect_workspace_files(options).map_err(Into::into)
}

pub fn inspect_workspace_entries(
    options: InspectWorkspaceEntriesOptions,
) -> Result<WorkspaceInspectEntries, WorkspaceError> {
    stringer_workspace_ops::inspect_workspace_entries(options).map_err(Into::into)
}

pub fn inspect_workspace_entry(
    options: InspectWorkspaceEntryOptions,
) -> Result<WorkspaceInspectEntry, WorkspaceError> {
    stringer_workspace_ops::inspect_workspace_entry(options).map_err(Into::into)
}

pub fn inspect_workspace_batch(
    options: InspectWorkspaceBatchOptions,
) -> Result<WorkspaceInspectBatch, WorkspaceError> {
    stringer_workspace_ops::inspect_workspace_batch(options).map_err(Into::into)
}

pub fn inspect_workspace_diagnostics(
    options: InspectWorkspaceDiagnosticsOptions,
) -> Result<WorkspaceInspectDiagnostics, WorkspaceError> {
    stringer_workspace_ops::inspect_workspace_diagnostics(options).map_err(Into::into)
}
