pub use stringer_workspace_ops::{
    BatchCount, BatchDetail, BatchDetailEntry, BatchExportFormat, BatchExportOptions,
    BatchExportSummary, BatchRead, BatchReadEntry, BatchSubmitAction, BatchSubmitEntry,
    BatchSubmitEntryResult, BatchSubmitOptions, BatchSubmitStatus, BatchSubmitSummary,
    ClaimBatchOptions, ClaimedBatch, CountBatchOptions, ReadBatchDetailOptions, ReadBatchOptions,
    ReleaseBatchOptions, ReleaseBatchSummary,
};

use crate::WorkspaceError;

pub fn count_batch(options: CountBatchOptions) -> Result<BatchCount, WorkspaceError> {
    stringer_workspace_ops::count_batch(options).map_err(Into::into)
}

pub fn claim_batch(options: ClaimBatchOptions) -> Result<ClaimedBatch, WorkspaceError> {
    stringer_workspace_ops::claim_batch(options).map_err(Into::into)
}

pub fn read_batch(options: ReadBatchOptions) -> Result<BatchRead, WorkspaceError> {
    stringer_workspace_ops::read_batch(options).map_err(Into::into)
}

pub fn read_batch_detail(options: ReadBatchDetailOptions) -> Result<BatchDetail, WorkspaceError> {
    stringer_workspace_ops::read_batch_detail(options).map_err(Into::into)
}

pub fn submit_batch(options: BatchSubmitOptions) -> Result<BatchSubmitSummary, WorkspaceError> {
    stringer_workspace_ops::submit_batch(options).map_err(Into::into)
}

pub fn export_batch_submission(
    options: BatchExportOptions,
) -> Result<BatchExportSummary, WorkspaceError> {
    stringer_workspace_ops::export_batch_submission(options).map_err(Into::into)
}

pub fn release_batch(options: ReleaseBatchOptions) -> Result<ReleaseBatchSummary, WorkspaceError> {
    stringer_workspace_ops::release_batch(options).map_err(Into::into)
}
