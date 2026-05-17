use camino::Utf8PathBuf;
use stringer_workspace_api::{BatchSubmitOptions, WorkspaceError};

use crate::app::{CliError, read_input};

pub(crate) fn read_batch_submit_input(
    workspace: &Utf8PathBuf,
    input: &Utf8PathBuf,
) -> Result<BatchSubmitOptions, CliError> {
    let text = read_input(input)?;
    let options = BatchSubmitOptions::from_submission_text(workspace.clone(), input, &text)
        .map_err(WorkspaceError::from)?;
    Ok(options)
}
