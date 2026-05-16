pub use stringer_workspace_ops::{
    NormalizeRuleEncoding, NormalizeWarning, NormalizeWorkspaceOptions, NormalizeWorkspaceSummary,
    WorkspaceNormalizeChange,
};

use crate::WorkspaceError;

pub fn normalize_workspace(
    options: NormalizeWorkspaceOptions,
) -> Result<NormalizeWorkspaceSummary, WorkspaceError> {
    stringer_workspace_ops::normalize_workspace(options).map_err(Into::into)
}
