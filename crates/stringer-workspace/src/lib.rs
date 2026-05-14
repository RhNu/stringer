#![forbid(unsafe_code)]

mod error;
mod operations;
mod package;
mod paths;
mod settings;

pub use error::WorkspaceError;
pub use operations::{
    ExportSummary, ExportTranslationsOptions, ImportSummary, ImportTranslationsOptions,
    WriteTarget, export_translations, import_translations,
};
pub use package::{
    SCHEMA_VERSION, TranslationManifest, TranslationManifestFile, TranslationRecord,
};
pub use settings::{
    LoadWorkspaceSettingsOptions, WorkspaceSettings, WorkspaceSettingsOverrides,
    default_config_path, game_release_name, language_name, load_workspace_settings,
    parse_game_release_name, parse_language_name,
};
