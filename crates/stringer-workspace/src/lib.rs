#![forbid(unsafe_code)]

mod batch;
mod error;
mod inspect;
mod operations;
mod paths;

pub use batch::{
    ApplyBatchPatchEntry, ApplyBatchPatchInput, ApplyBatchPatchOptions, ApplyBatchPatchSummary,
    BatchCount, ClaimBatchOptions, ClaimedBatch, ClaimedBatchEntry, CountBatchOptions,
    ReleaseBatchOptions, ReleaseBatchSummary, apply_batch_patch, claim_batch, count_batch,
    release_batch,
};
pub use error::WorkspaceError;
pub use inspect::{
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions, WorkspaceInspectBatch,
    WorkspaceInspectDiagnostic, WorkspaceInspectDiagnostics, WorkspaceInspectEntries,
    WorkspaceInspectEntry, WorkspaceInspectFiles, inspect_workspace_batch,
    inspect_workspace_diagnostics, inspect_workspace_entries, inspect_workspace_entry,
    inspect_workspace_files,
};
pub use operations::{
    ExportSummary, ExportTranslationsOptions, ImportSummary, ImportTranslationsOptions,
    export_translations, import_translations,
};
pub use stringer_pipeline::PipelineEntryKind;
pub use stringer_workspace_core::{
    LoadWorkspaceSettingsOptions, SCHEMA_VERSION, TranslationManifest, TranslationManifestFile,
    TranslationMeta, TranslationRecord, WorkspaceSettings, WorkspaceSettingsOverrides,
    default_config_path, game_release_name, language_name, load_global_knowledge_root,
    load_workspace_settings, parse_game_release_name, parse_language_name, read_workspace_settings,
    read_workspace_source_root,
};
