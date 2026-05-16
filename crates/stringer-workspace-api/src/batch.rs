pub use stringer_workspace_ops::{
    ApplyBatchPatchEntry, ApplyBatchPatchInput, ApplyBatchPatchOptions, ApplyBatchPatchSummary,
    BatchCount, ClaimBatchOptions, ClaimedBatch, ClaimedBatchEntry, CountBatchOptions,
    ReleaseBatchOptions, ReleaseBatchSummary,
};

use crate::WorkspaceError;

pub fn count_batch(options: CountBatchOptions) -> Result<BatchCount, WorkspaceError> {
    stringer_workspace_ops::count_batch(options).map_err(Into::into)
}

pub fn claim_batch(options: ClaimBatchOptions) -> Result<ClaimedBatch, WorkspaceError> {
    stringer_workspace_ops::claim_batch(options).map_err(Into::into)
}

pub fn apply_batch_patch(
    options: ApplyBatchPatchOptions,
) -> Result<ApplyBatchPatchSummary, WorkspaceError> {
    stringer_workspace_ops::apply_batch_patch(options).map_err(Into::into)
}

pub fn release_batch(options: ReleaseBatchOptions) -> Result<ReleaseBatchSummary, WorkspaceError> {
    stringer_workspace_ops::release_batch(options).map_err(Into::into)
}
