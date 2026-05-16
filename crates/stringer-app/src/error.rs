use camino::Utf8PathBuf;
use serde_json::{Value, json};
use stringer_adapt::AdaptError;
use stringer_knowledge::KnowledgeError;
use stringer_workspace_api::WorkspaceError;
use stringer_workspace_core::WorkspaceCoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),

    #[error(transparent)]
    Knowledge(#[from] KnowledgeError),

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
            Self::Knowledge(error) => knowledge_error_code(error),
            Self::Adapt(error) => adapt_error_code(error),
            Self::Serialize { .. } => "app.serialize",
        }
    }

    pub fn details(&self) -> Value {
        match self {
            Self::Workspace(error) => workspace_error_details(error),
            Self::Knowledge(error) => knowledge_error_details(error),
            Self::Adapt(error) => adapt_error_details(error),
            Self::Serialize { message, .. } => json!({ "message": message }),
        }
    }
}

impl From<WorkspaceCoreError> for AppError {
    fn from(source: WorkspaceCoreError) -> Self {
        Self::Workspace(source.into())
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
        WorkspaceError::ExtractionFilter(_) => "workspace.extraction_filter",
        WorkspaceError::JsonLine { .. } => "workspace.json_line",
        WorkspaceError::Json { .. } => "workspace.json",
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
        WorkspaceError::InvalidOutputRoot { .. } => "workspace.invalid_output_root",
        WorkspaceError::Reader(_) => "workspace.reader",
        WorkspaceError::Plugin(_) => "workspace.plugin",
        WorkspaceError::Pex(_) => "workspace.pex",
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
        | WorkspaceError::LegacyTranslationWorkspace { path }
        | WorkspaceError::WorkspaceLocked { path } => json!({ "path": json_path(path) }),
        WorkspaceError::JsonLine { path, line, .. } => {
            json!({ "path": json_path(path), "line": line })
        }
        WorkspaceError::MissingSetting { name } => json!({ "name": name }),
        WorkspaceError::InvalidSetting { name, value } => {
            json!({ "name": name, "value": value })
        }
        WorkspaceError::ExtractionFilter(error) => json!({ "message": error.to_string() }),
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
        WorkspaceError::BatchNotFound { batch_id } => json!({
            "batch_id": batch_id,
            "recovery": "claim_fresh_batch",
        }),
        WorkspaceError::BatchEntryNotClaimed { batch_id, id } => {
            json!({
                "batch_id": batch_id,
                "id": id,
                "recovery": "inspect_batch_from_offset_0",
            })
        }
        WorkspaceError::DuplicateOutputPath { path } => json!({ "path": path }),
        WorkspaceError::InvalidLogicalPath { path, message } => {
            json!({ "path": path, "message": message })
        }
        WorkspaceError::InvalidOutputRoot { root, message } => {
            json!({ "root": json_path(root), "message": message })
        }
        WorkspaceError::CurrentDirectory { .. }
        | WorkspaceError::Reader(_)
        | WorkspaceError::Plugin(_)
        | WorkspaceError::Pex(_)
        | WorkspaceError::Scaleform(_)
        | WorkspaceError::Bundle(_) => json!({}),
    }
}

fn knowledge_error_code(error: &KnowledgeError) -> &'static str {
    match error {
        KnowledgeError::Core(error) => workspace_core_error_code(error),
        KnowledgeError::KnowledgeTermsToml { .. } => "workspace.knowledge_terms_toml",
        KnowledgeError::InvalidKnowledgeTermsToml { .. } => {
            "workspace.invalid_knowledge_terms_toml"
        }
        KnowledgeError::KnowledgeTermNotFound { .. } => "workspace.knowledge_term_not_found",
        KnowledgeError::InvalidKnowledgeTermScope { .. } => {
            "workspace.invalid_knowledge_term_scope"
        }
        KnowledgeError::InvalidKnowledgeTermFile { .. } => "workspace.invalid_knowledge_term_file",
        KnowledgeError::InvalidLookupRegex { .. } => "workspace.invalid_lookup_regex",
        KnowledgeError::Sqlite { .. } => "workspace.sqlite",
        KnowledgeError::CandidateIndex { .. } => "workspace.candidate_index",
        KnowledgeError::Pipeline(_) => "workspace.pipeline",
    }
}

fn knowledge_error_details(error: &KnowledgeError) -> Value {
    match error {
        KnowledgeError::Core(error) => workspace_core_error_details(error),
        KnowledgeError::KnowledgeTermsToml { path, .. } | KnowledgeError::Sqlite { path, .. } => {
            json!({ "path": json_path(path) })
        }
        KnowledgeError::InvalidKnowledgeTermsToml { path, message }
        | KnowledgeError::InvalidKnowledgeTermFile { path, message } => {
            json!({ "path": json_path(path), "message": message })
        }
        KnowledgeError::KnowledgeTermNotFound { path, id } => {
            json!({ "path": json_path(path), "id": id })
        }
        KnowledgeError::InvalidKnowledgeTermScope { id, key } => {
            json!({ "id": id, "key": key })
        }
        KnowledgeError::InvalidLookupRegex { pattern, .. } => json!({ "pattern": pattern }),
        KnowledgeError::CandidateIndex { message } => json!({ "message": message }),
        KnowledgeError::Pipeline(_) => json!({}),
    }
}

fn workspace_core_error_code(error: &WorkspaceCoreError) -> &'static str {
    match error {
        WorkspaceCoreError::ReadFile { .. } => "workspace.read_file",
        WorkspaceCoreError::WriteFile { .. } => "workspace.write_file",
        WorkspaceCoreError::CurrentDirectory { .. } => "workspace.current_directory",
        WorkspaceCoreError::ConfigToml { .. } => "workspace.config_toml",
        WorkspaceCoreError::Toml { .. } => "workspace.toml",
        WorkspaceCoreError::MissingSetting { .. } => "workspace.missing_setting",
        WorkspaceCoreError::InvalidSetting { .. } => "workspace.invalid_setting",
        WorkspaceCoreError::ExtractionFilter(_) => "workspace.extraction_filter",
        WorkspaceCoreError::JsonLine { .. } => "workspace.json_line",
        WorkspaceCoreError::Json { .. } => "workspace.json",
        WorkspaceCoreError::UnsupportedTranslationSchema { .. } => {
            "workspace.unsupported_translation_schema"
        }
        WorkspaceCoreError::LegacyTranslationWorkspace { .. } => {
            "workspace.legacy_translation_workspace"
        }
        WorkspaceCoreError::WorkspaceLocked { .. } => "workspace.locked",
        WorkspaceCoreError::InvalidTranslationPackagePath { .. } => {
            "workspace.invalid_translation_package_path"
        }
        WorkspaceCoreError::InvalidLogicalPath { .. } => "workspace.invalid_logical_path",
        WorkspaceCoreError::DuplicateTranslationId { .. } => "workspace.duplicate_translation_id",
        WorkspaceCoreError::BatchNotFound { .. } => "workspace.batch_not_found",
    }
}

fn workspace_core_error_details(error: &WorkspaceCoreError) -> Value {
    match error {
        WorkspaceCoreError::ReadFile { path, .. }
        | WorkspaceCoreError::WriteFile { path, .. }
        | WorkspaceCoreError::ConfigToml { path, .. }
        | WorkspaceCoreError::Toml { path, .. }
        | WorkspaceCoreError::Json { path, .. }
        | WorkspaceCoreError::LegacyTranslationWorkspace { path }
        | WorkspaceCoreError::WorkspaceLocked { path } => json!({ "path": json_path(path) }),
        WorkspaceCoreError::JsonLine { path, line, .. } => {
            json!({ "path": json_path(path), "line": line })
        }
        WorkspaceCoreError::MissingSetting { name } => json!({ "name": name }),
        WorkspaceCoreError::InvalidSetting { name, value } => {
            json!({ "name": name, "value": value })
        }
        WorkspaceCoreError::ExtractionFilter(error) => json!({ "message": error.to_string() }),
        WorkspaceCoreError::UnsupportedTranslationSchema { path, version } => {
            json!({ "path": json_path(path), "version": version })
        }
        WorkspaceCoreError::InvalidTranslationPackagePath { path, message } => {
            json!({ "path": path, "message": message })
        }
        WorkspaceCoreError::InvalidLogicalPath { path, message } => {
            json!({ "path": path, "message": message })
        }
        WorkspaceCoreError::DuplicateTranslationId { path, id } => {
            json!({ "path": json_path(path), "id": id })
        }
        WorkspaceCoreError::BatchNotFound { batch_id } => json!({
            "batch_id": batch_id,
            "recovery": "claim_fresh_batch",
        }),
        WorkspaceCoreError::CurrentDirectory { .. } => json!({}),
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
