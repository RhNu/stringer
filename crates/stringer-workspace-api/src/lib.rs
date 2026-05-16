#![forbid(unsafe_code)]

mod batch;
mod error;
mod inspect;
mod normalize;
mod operations;
mod paths;

pub use batch::{
    BatchCount, BatchDetail, BatchDetailEntry, BatchExportFormat, BatchExportOptions,
    BatchExportSummary, BatchRead, BatchReadEntry, BatchSubmitAction, BatchSubmitEntry,
    BatchSubmitEntryResult, BatchSubmitOptions, BatchSubmitStatus, BatchSubmitSummary,
    ClaimBatchOptions, ClaimedBatch, CountBatchOptions, ReadBatchDetailOptions, ReadBatchOptions,
    ReleaseBatchOptions, ReleaseBatchSummary, claim_batch, count_batch, export_batch_submission,
    read_batch, read_batch_detail, release_batch, submit_batch,
};
pub use error::WorkspaceError;
pub use inspect::{
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceDiagnosticsOptions,
    InspectWorkspaceEntriesOptions, InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions,
    WorkspaceInspectDiagnostic, WorkspaceInspectDiagnostics, WorkspaceInspectEntries,
    WorkspaceInspectEntry, WorkspaceInspectFiles, inspect_workspace_diagnostics,
    inspect_workspace_entries, inspect_workspace_entry, inspect_workspace_files,
};
pub use normalize::{
    NormalizeRuleEncoding, NormalizeWarning, NormalizeWorkspaceOptions, NormalizeWorkspaceSummary,
    WorkspaceNormalizeChange, normalize_workspace,
};
pub use operations::{
    ExportSummary, ExportTranslationsOptions, ImportSummary, ImportTranslationsOptions,
    export_translations, import_translations,
};
pub use stringer_pipeline::PipelineEntryKind;
pub use stringer_workspace_core::{
    GlobalConfigSource, LoadWorkspaceSettingsOptions, SCHEMA_VERSION, TranslationManifest,
    TranslationManifestFile, TranslationMeta, TranslationRecord, WorkspaceSettings,
    WorkspaceSettingsOverrides, default_config_path, game_release_name,
    global_knowledge_root_from_source, language_name, load_global_knowledge_root,
    load_workspace_settings, parse_game_release_name, parse_language_name, read_workspace_settings,
    read_workspace_source_root, with_global_knowledge_defaults,
};
