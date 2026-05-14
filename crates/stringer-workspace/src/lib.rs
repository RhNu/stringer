#![forbid(unsafe_code)]

mod error;
mod knowledge;
mod operations;
mod package;
mod paths;
mod settings;

pub use error::WorkspaceError;
pub use knowledge::{
    AnnotateTranslationsOptions, KnowledgeLookup, KnowledgeSummary, LookupKnowledgeOptions,
    ValidateTranslationsOptions, annotate_translations, load_knowledge_layers, lookup_knowledge,
    validate_translations,
};
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
pub use stringer_pipeline::PipelineEntryKind;
