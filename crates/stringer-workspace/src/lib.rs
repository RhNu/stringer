#![forbid(unsafe_code)]

mod error;
mod knowledge;
mod knowledge_index;
mod knowledge_lookup;
mod operations;
mod package;
mod paths;
mod settings;

pub use error::WorkspaceError;
pub use knowledge::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeIndexSummary,
    KnowledgeLayerOverrides, KnowledgeSummary, LoadKnowledgeLayersOptions, LoadedKnowledgeLayers,
    LookupKnowledgeOptions, ValidateTranslationsOptions, annotate_translations,
    build_knowledge_index, load_knowledge_layers, lookup_knowledge, validate_translations,
};
pub use knowledge_lookup::{
    KnowledgeLookup, KnowledgeLookupResult, LookupKnowledgeField, LookupKnowledgeMode,
    LookupKnowledgeSource,
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
    default_config_path, game_release_name, language_name, load_global_knowledge_root,
    load_workspace_settings, parse_game_release_name, parse_language_name,
};
pub use stringer_pipeline::PipelineEntryKind;
