use camino::Utf8PathBuf;
use serde_json::{Value, json};
use stringer_adapt::AdaptError;
use stringer_workspace::WorkspaceError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),

    #[error(transparent)]
    Adapt(#[from] AdaptError),

    #[error("failed to serialize `{message}`: {source}")]
    Serialize {
        message: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Workspace(error) => workspace_error_code(error),
            Self::Adapt(error) => adapt_error_code(error),
            Self::Serialize { .. } => "app.serialize",
        }
    }

    pub fn details(&self) -> Value {
        match self {
            Self::Workspace(error) => workspace_error_details(error),
            Self::Adapt(error) => adapt_error_details(error),
            Self::Serialize { message, .. } => json!({ "message": message }),
        }
    }
}

pub(crate) fn serialize_value<T: serde::Serialize>(
    message: &'static str,
    value: T,
) -> Result<Value, AppError> {
    serde_json::to_value(value).map_err(|source| AppError::Serialize { message, source })
}

fn workspace_error_code(error: &WorkspaceError) -> &'static str {
    match error {
        WorkspaceError::ReadFile { .. } => "workspace.read_file",
        WorkspaceError::WriteFile { .. } => "workspace.write_file",
        WorkspaceError::CurrentDirectory { .. } => "workspace.current_directory",
        WorkspaceError::ConfigToml { .. } => "workspace.config_toml",
        WorkspaceError::Toml { .. } => "workspace.toml",
        WorkspaceError::MissingSetting { .. } => "workspace.missing_setting",
        WorkspaceError::InvalidSetting { .. } => "workspace.invalid_setting",
        WorkspaceError::InvalidLookupRegex { .. } => "workspace.invalid_lookup_regex",
        WorkspaceError::JsonLine { .. } => "workspace.json_line",
        WorkspaceError::Json { .. } => "workspace.json",
        WorkspaceError::Sqlite { .. } => "workspace.sqlite",
        WorkspaceError::UnsupportedTranslationSchema { .. } => {
            "workspace.unsupported_translation_schema"
        }
        WorkspaceError::LegacyTranslationWorkspace { .. } => {
            "workspace.legacy_translation_workspace"
        }
        WorkspaceError::WorkspaceLocked { .. } => "workspace.locked",
        WorkspaceError::InvalidTranslationPackagePath { .. } => {
            "workspace.invalid_translation_package_path"
        }
        WorkspaceError::DuplicateTranslationId { .. } => "workspace.duplicate_translation_id",
        WorkspaceError::UnknownTranslationId { .. } => "workspace.unknown_translation_id",
        WorkspaceError::BatchNotFound { .. } => "workspace.batch_not_found",
        WorkspaceError::DuplicateBatchPatchId { .. } => "workspace.duplicate_batch_patch_id",
        WorkspaceError::MissingBatchPatchTranslation { .. } => {
            "workspace.missing_batch_patch_translation"
        }
        WorkspaceError::BatchEntryNotClaimed { .. } => "workspace.batch_entry_not_claimed",
        WorkspaceError::DuplicateOutputPath { .. } => "workspace.duplicate_output_path",
        WorkspaceError::InvalidLogicalPath { .. } => "workspace.invalid_logical_path",
        WorkspaceError::InvalidOverrideRoot { .. } => "workspace.invalid_override_root",
        WorkspaceError::Reader(_) => "workspace.reader",
        WorkspaceError::Plugin(_) => "workspace.plugin",
        WorkspaceError::Pex(_) => "workspace.pex",
        WorkspaceError::Pipeline(_) => "workspace.pipeline",
        WorkspaceError::Scaleform(_) => "workspace.scaleform",
        WorkspaceError::Bundle(_) => "workspace.bundle",
    }
}

fn workspace_error_details(error: &WorkspaceError) -> Value {
    match error {
        WorkspaceError::ReadFile { path, .. }
        | WorkspaceError::WriteFile { path, .. }
        | WorkspaceError::ConfigToml { path, .. }
        | WorkspaceError::Toml { path, .. }
        | WorkspaceError::Json { path, .. }
        | WorkspaceError::Sqlite { path, .. }
        | WorkspaceError::LegacyTranslationWorkspace { path }
        | WorkspaceError::WorkspaceLocked { path } => json!({ "path": json_path(path) }),
        WorkspaceError::JsonLine { path, line, .. } => {
            json!({ "path": json_path(path), "line": line })
        }
        WorkspaceError::MissingSetting { name } => json!({ "name": name }),
        WorkspaceError::InvalidSetting { name, value } => {
            json!({ "name": name, "value": value })
        }
        WorkspaceError::InvalidLookupRegex { pattern, .. } => json!({ "pattern": pattern }),
        WorkspaceError::UnsupportedTranslationSchema { path, version } => {
            json!({ "path": json_path(path), "version": version })
        }
        WorkspaceError::InvalidTranslationPackagePath { path, message } => {
            json!({ "path": path, "message": message })
        }
        WorkspaceError::DuplicateTranslationId { path, id } => {
            json!({ "path": json_path(path), "id": id })
        }
        WorkspaceError::UnknownTranslationId { id }
        | WorkspaceError::DuplicateBatchPatchId { id }
        | WorkspaceError::MissingBatchPatchTranslation { id } => json!({ "id": id }),
        WorkspaceError::BatchNotFound { batch_id } => json!({ "batch_id": batch_id }),
        WorkspaceError::BatchEntryNotClaimed { batch_id, id } => {
            json!({ "batch_id": batch_id, "id": id })
        }
        WorkspaceError::DuplicateOutputPath { path } => json!({ "path": path }),
        WorkspaceError::InvalidLogicalPath { path, message } => {
            json!({ "path": path, "message": message })
        }
        WorkspaceError::InvalidOverrideRoot { root, message } => {
            json!({ "root": json_path(root), "message": message })
        }
        WorkspaceError::CurrentDirectory { .. }
        | WorkspaceError::Reader(_)
        | WorkspaceError::Plugin(_)
        | WorkspaceError::Pex(_)
        | WorkspaceError::Pipeline(_)
        | WorkspaceError::Scaleform(_)
        | WorkspaceError::Bundle(_) => json!({}),
    }
}

fn adapt_error_code(error: &AdaptError) -> &'static str {
    match error {
        AdaptError::ReadFile { .. } => "adapt.read_file",
        AdaptError::WriteFile { .. } => "adapt.write_file",
        AdaptError::Json { .. } => "adapt.json",
        AdaptError::Malformed { .. } => "adapt.malformed",
    }
}

fn adapt_error_details(error: &AdaptError) -> Value {
    match error {
        AdaptError::ReadFile { path, .. }
        | AdaptError::WriteFile { path, .. }
        | AdaptError::Json { path, .. } => json!({ "path": json_path(path) }),
        AdaptError::Malformed {
            path,
            format,
            message,
        } => json!({ "path": json_path(path), "format": format, "message": message }),
    }
}

fn json_path(path: &Utf8PathBuf) -> String {
    path.as_str().replace('\\', "/")
}
