use camino::Utf8PathBuf;
use stringer_workspace::WorkspaceError;

pub(crate) fn path(value: String) -> Utf8PathBuf {
    Utf8PathBuf::from(value)
}

pub(crate) fn workspace_config_path(root: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    let candidate = root.join("stringer.toml");
    candidate.exists().then_some(candidate)
}

pub(crate) fn workspace_or_current(
    workspace: Option<String>,
) -> Result<Utf8PathBuf, WorkspaceError> {
    if let Some(workspace) = workspace {
        return Ok(path(workspace));
    }
    let current =
        std::env::current_dir().map_err(|source| WorkspaceError::CurrentDirectory { source })?;
    Utf8PathBuf::from_path_buf(current).map_err(|path| WorkspaceError::InvalidLogicalPath {
        path: path.display().to_string(),
        message: "current directory is not valid UTF-8".to_string(),
    })
}

pub(crate) fn initialized_workspace_or_current(
    workspace: Option<String>,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let workspace = workspace_or_current(workspace)?;
    if workspace.join("workspace.json").exists() {
        return Ok(workspace);
    }
    Err(WorkspaceError::InvalidTranslationPackagePath {
        path: workspace.to_string(),
        message: "workspace.json was not found; run workspace open first".to_string(),
    })
}

pub(crate) fn default_output_path(workspace: &Utf8PathBuf) -> Utf8PathBuf {
    workspace.join("output")
}

pub(crate) fn default_adapt_memory_path(
    root: &Utf8PathBuf,
    input: &Utf8PathBuf,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let file_name = input
        .file_name()
        .ok_or_else(|| WorkspaceError::InvalidLogicalPath {
            path: input.to_string(),
            message: "adapt input path must include a file name".to_string(),
        })?;
    Ok(root
        .join("memory")
        .join("adapt")
        .join(format!("{file_name}.jsonl")))
}
