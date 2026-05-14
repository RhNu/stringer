#![forbid(unsafe_code)]

mod error;
mod jsonl;
mod operations;
mod paths;
mod settings;

pub use error::WorkspaceError;
pub use jsonl::{SCHEMA_VERSION, TranslationRecord};
pub use operations::{
    ExportSummary, ExportTranslationsOptions, ImportSummary, ImportTranslationsOptions,
    WriteTarget, export_translation_jsonl, import_translation_jsonl,
};
pub use settings::{
    LoadWorkspaceSettingsOptions, WorkspaceSettings, WorkspaceSettingsOverrides,
    default_config_path, game_release_name, language_name, load_workspace_settings,
    parse_game_release_name, parse_language_name,
};
