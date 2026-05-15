#![forbid(unsafe_code)]

mod batch;
mod error;
mod fsutil;
mod knowledge;
mod knowledge_index;
mod knowledge_lookup;
mod lock;
mod operations;
mod package;
mod paths;
mod settings;

pub use batch::{
    ApplyBatchPatchEntry, ApplyBatchPatchInput, ApplyBatchPatchOptions, ApplyBatchPatchSummary,
    BatchCount, ClaimBatchOptions, ClaimedBatch, ClaimedBatchEntry, CountBatchOptions,
    ReleaseBatchOptions, ReleaseBatchSummary, apply_batch_patch, claim_batch, count_batch,
    release_batch,
};
pub use error::WorkspaceError;
pub use knowledge::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeIndexSummary,
    KnowledgeSummary, LoadKnowledgeLayersOptions, LoadedKnowledgeLayers, LookupKnowledgeOptions,
    ValidateTranslationsOptions, annotate_translations, build_knowledge_index,
    load_knowledge_layers, lookup_knowledge, validate_translations,
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
    SCHEMA_VERSION, TranslationManifest, TranslationManifestFile, TranslationMeta,
    TranslationRecord,
};
pub use settings::{
    LoadWorkspaceSettingsOptions, WorkspaceSettings, WorkspaceSettingsOverrides,
    default_config_path, game_release_name, language_name, load_global_knowledge_root,
    load_workspace_settings, parse_game_release_name, parse_language_name,
};
pub use stringer_pipeline::PipelineEntryKind;
