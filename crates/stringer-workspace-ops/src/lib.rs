#![forbid(unsafe_code)]

mod batch;
mod batch_packet;
mod error;
mod inspect;
mod normalize;

pub use batch::{
    ApplyBatchPatchEntry, ApplyBatchPatchInput, ApplyBatchPatchOptions, ApplyBatchPatchSummary,
    BatchCount, ClaimBatchOptions, ClaimedBatch, CountBatchOptions, ReleaseBatchOptions,
    ReleaseBatchSummary, apply_batch_patch, claim_batch, count_batch, release_batch,
};
pub use batch_packet::{
    BatchDetail, BatchDetailEntry, BatchExportFormat, BatchExportOptions, BatchExportSummary,
    BatchRead, BatchReadEntry, BatchSubmitAction, BatchSubmitEntry, BatchSubmitEntryResult,
    BatchSubmitOptions, BatchSubmitStatus, BatchSubmitSummary, ReadBatchDetailOptions,
    ReadBatchOptions, export_batch_patch, read_batch, read_batch_detail, submit_batch,
};
pub use error::WorkspaceOpsError;
pub use inspect::{
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, WorkspaceInspectBatch,
    WorkspaceInspectDiagnostic, WorkspaceInspectDiagnostics, WorkspaceInspectEntries,
    WorkspaceInspectEntry, WorkspaceInspectFiles, inspect_workspace_batch,
    inspect_workspace_diagnostics, inspect_workspace_entries, inspect_workspace_entry,
    inspect_workspace_files,
};
pub use normalize::{
    NormalizeRuleEncoding, NormalizeWarning, NormalizeWorkspaceOptions, NormalizeWorkspaceSummary,
    WorkspaceNormalizeChange, normalize_workspace,
};
