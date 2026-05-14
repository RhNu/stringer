use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use stringer_core::{FileAsset, FileBundle, StringEntryBundle, StringEntryView};
use stringer_pex::{PexStringBundle, ReadPexOptions, read_pex_strings, write_pex_strings};
use stringer_plugin::{
    LocalizationBundle, ReadOptions, WriteOptions, read_localization, write_localization,
};
use stringer_reader::{ReadModOptions, read_mod_root};
use stringer_scaleform::{
    ScaleformTranslationBundle, read_scaleform_translations, write_scaleform_translations,
};
use tracing::debug;

use crate::WorkspaceError;
use crate::package::{
    PackagedTranslationRecord, external_entry_id, packaged_record_from_entry,
    read_translation_package, write_translation_package,
};
use crate::paths::{changed_assets, ensure_override_root_outside_source, write_override_assets};
use crate::settings::WorkspaceSettings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTranslationsOptions {
    pub root: Utf8PathBuf,
    pub out: Utf8PathBuf,
    pub settings: WorkspaceSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTranslationsOptions {
    pub root: Utf8PathBuf,
    pub translations: Utf8PathBuf,
    pub target: WriteTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteTarget {
    OverrideDirectory { root: Utf8PathBuf },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportSummary {
    pub entries: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub applied_entries: usize,
    pub written_files: usize,
}

enum EditableBundle {
    Plugin(LocalizationBundle),
    Pex(PexStringBundle),
    Scaleform(ScaleformTranslationBundle),
}

impl EditableBundle {
    fn collect_records(
        &self,
        settings: &WorkspaceSettings,
        records: &mut Vec<PackagedTranslationRecord>,
    ) {
        match self {
            Self::Plugin(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
            Self::Pex(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
            Self::Scaleform(bundle) => {
                records.extend(
                    bundle
                        .string_entries()
                        .iter()
                        .map(|entry| packaged_record_from_entry(entry, settings)),
                );
            }
        }
    }

    fn apply_translations(&mut self, translations: &mut BTreeMap<String, String>) -> usize {
        match self {
            Self::Plugin(bundle) => apply_entries(bundle.string_entries_mut(), translations),
            Self::Pex(bundle) => apply_entries(bundle.string_entries_mut(), translations),
            Self::Scaleform(bundle) => apply_entries(bundle.string_entries_mut(), translations),
        }
    }

    async fn write(self, settings: &WorkspaceSettings) -> Result<Vec<FileAsset>, WorkspaceError> {
        match self {
            Self::Plugin(bundle) => {
                let output = write_localization(
                    bundle,
                    WriteOptions::new(settings.game_release, settings.asset_language),
                )
                .await?;
                Ok(output.into_files())
            }
            Self::Pex(bundle) => Ok(vec![write_pex_strings(bundle)?]),
            Self::Scaleform(bundle) => Ok(write_scaleform_translations(bundle)?.into_files()),
        }
    }
}

pub async fn export_translations(
    options: ExportTranslationsOptions,
) -> Result<ExportSummary, WorkspaceError> {
    let read = read_mod_root(&options.root, ReadModOptions::default())?;
    let bundles = read_editable_bundles(&read.files, &options.settings).await?;
    let mut records = Vec::new();
    for bundle in &bundles {
        bundle.collect_records(&options.settings, &mut records);
    }
    records
        .sort_by(|left, right| (&left.file, &left.record.id).cmp(&(&right.file, &right.record.id)));
    write_translation_package(&options.out, &options.settings, &records)?;
    debug!(entries = records.len(), "exported translation package");
    Ok(ExportSummary {
        entries: records.len(),
    })
}

pub async fn import_translations(
    options: ImportTranslationsOptions,
) -> Result<ImportSummary, WorkspaceError> {
    let (settings, mut translations) = read_translation_package(&options.translations)?;
    let read = read_mod_root(&options.root, ReadModOptions::default())?;
    let mut bundles = read_editable_bundles(&read.files, &settings).await?;
    let mut applied_entries = 0;
    for bundle in &mut bundles {
        applied_entries += bundle.apply_translations(&mut translations);
    }
    if let Some(id) = translations.keys().next() {
        return Err(WorkspaceError::UnknownTranslationId { id: id.clone() });
    }

    let mut written_candidates = Vec::new();
    for bundle in bundles {
        written_candidates.extend(bundle.write(&settings).await?);
    }
    let changed = changed_assets(&read.files, written_candidates)?;
    let written_files = match options.target {
        WriteTarget::OverrideDirectory { root } => {
            ensure_override_root_outside_source(&options.root, &root)?;
            write_override_assets(&root, &changed)?
        }
    };
    debug!(
        applied_entries,
        written_files, "imported translation package"
    );
    Ok(ImportSummary {
        applied_entries,
        written_files,
    })
}

async fn read_editable_bundles(
    files: &FileBundle,
    settings: &WorkspaceSettings,
) -> Result<Vec<EditableBundle>, WorkspaceError> {
    let mut bundles = Vec::new();
    let string_assets = files.strings().cloned().collect::<Vec<_>>();

    for plugin in files.plugins().cloned() {
        let mut plugin_files = Vec::with_capacity(string_assets.len() + 1);
        plugin_files.push(plugin);
        plugin_files.extend(string_assets.iter().cloned());
        let bundle = read_localization(
            FileBundle::new(plugin_files),
            ReadOptions::new(settings.game_release, settings.asset_language),
        )
        .await?;
        bundles.push(EditableBundle::Plugin(bundle));
    }

    for asset in files.pex().cloned() {
        bundles.push(EditableBundle::Pex(read_pex_strings(
            asset,
            ReadPexOptions::default(),
        )?));
    }

    let scaleform = read_scaleform_translations(files.clone(), settings.asset_language)?;
    if !scaleform.string_entries().is_empty() {
        bundles.push(EditableBundle::Scaleform(scaleform));
    }

    Ok(bundles)
}

fn apply_entries(
    entries: &mut [impl StringEntryView],
    translations: &mut BTreeMap<String, String>,
) -> usize {
    let mut applied = 0;
    for entry in entries {
        let id = external_entry_id(entry.string_entry());
        if let Some(text) = translations.remove(&id) {
            entry.string_entry_mut().set_text(text);
            applied += 1;
        }
    }
    applied
}
