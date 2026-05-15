#![forbid(unsafe_code)]

mod batch_claims;
mod error;
pub mod fsutil;
pub mod lock;
mod package;
mod settings;

pub use batch_claims::{
    BatchFile, BatchScope, batch_entry_ids, claimed_entry_batches, claimed_entry_ids,
    read_batch_file, validate_batch_id,
};
pub use error::WorkspaceCoreError;
pub use lock::{WorkspaceLock, unix_ms};
pub use package::{
    PackagedTranslationRecord, SCHEMA_VERSION, TranslationFileKey, TranslationManifest,
    TranslationManifestFile, TranslationMeta, TranslationPackageFileRecords,
    TranslationPackageRecords, TranslationRecord, external_entry_id, packaged_record_from_entry,
    read_translation_manifest, read_translation_manifest_files, read_translation_package,
    read_translation_package_records, read_translation_package_records_filtered,
    read_workspace_settings, read_workspace_source_root,
    visit_translation_package_records_filtered, write_translation_package,
    write_translation_package_records,
};
pub use settings::{
    LoadWorkspaceSettingsOptions, WorkspaceSettings, WorkspaceSettingsOverrides,
    default_config_path, game_release_name, language_name, load_global_knowledge_root,
    load_workspace_settings, parse_game_release_name, parse_language_name,
};
