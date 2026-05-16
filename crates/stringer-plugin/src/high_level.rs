use std::collections::BTreeMap;

use stringer_core::{
    FileBundle, PluginStringMetadata, PluginStringStorage, StringEntry, StringEntryBundle,
    StringEntryContext, StringEntrySource, StringEntryView,
};
use stringer_extraction_filter::ExtractionFilterSet;
use tokio::task;
use tracing::{debug, instrument, trace};

use crate::filter::{PluginStringFilter, PluginStringFilterInput};
use crate::{
    GameRelease, Language, ParsePluginOptions, ParsedPlugin, PluginError, StringsFile, StringsKind,
    parse_plugin_file, parse_strings_file, write_plugin_file, write_strings_file,
};

#[derive(Debug, Clone)]
pub struct ReadOptions {
    release: GameRelease,
    language: Language,
    extraction_filters: ExtractionFilterSet,
}

impl ReadOptions {
    pub fn new(release: GameRelease, language: Language) -> Self {
        Self {
            release,
            language,
            extraction_filters: ExtractionFilterSet::default(),
        }
    }

    pub fn with_extraction_filters(mut self, filters: ExtractionFilterSet) -> Self {
        self.extraction_filters = filters;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WriteOptions {
    release: GameRelease,
    language: Language,
}

impl WriteOptions {
    pub fn new(release: GameRelease, language: Language) -> Self {
        Self { release, language }
    }
}

#[derive(Debug, Clone)]
pub struct LocalizationBundle {
    plugin: ParsedPlugin,
    strings: BTreeMap<StringsKind, StringsFile>,
    entries: Vec<LocalizationEntry>,
    bindings: Vec<PluginEntryBinding>,
}

impl LocalizationBundle {
    pub fn entries(&self) -> &[LocalizationEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [LocalizationEntry] {
        &mut self.entries
    }
}

impl StringEntryBundle for LocalizationBundle {
    type Entry = LocalizationEntry;

    fn string_entries(&self) -> &[LocalizationEntry] {
        &self.entries
    }

    fn string_entries_mut(&mut self) -> &mut [LocalizationEntry] {
        &mut self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationEntry {
    entry: StringEntry,
}

impl LocalizationEntry {
    fn new(entry: StringEntry) -> Self {
        Self { entry }
    }

    pub fn id(&self) -> &str {
        self.entry.id()
    }

    pub fn text(&self) -> &str {
        self.entry.text()
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.entry.set_text(text);
    }

    pub fn source(&self) -> &StringEntrySource {
        self.entry.source()
    }

    pub fn source_mut(&mut self) -> &mut StringEntrySource {
        self.entry.source_mut()
    }

    pub fn is_dirty(&self) -> bool {
        self.entry.is_dirty()
    }

    pub fn strings_kind(&self) -> StringsKind {
        let StringEntrySource::Plugin(metadata) = self.entry.source() else {
            return StringsKind::Normal;
        };
        match metadata.strings_kind.as_str() {
            "DLSTRINGS" => StringsKind::Dl,
            "ILSTRINGS" => StringsKind::Il,
            _ => StringsKind::Normal,
        }
    }

    pub fn string_id(&self) -> Option<u32> {
        let StringEntrySource::Plugin(metadata) = self.entry.source() else {
            return None;
        };
        metadata.string_id
    }
}

impl StringEntryView for LocalizationEntry {
    fn string_entry(&self) -> &StringEntry {
        &self.entry
    }

    fn string_entry_mut(&mut self) -> &mut StringEntry {
        &mut self.entry
    }
}

#[derive(Debug, Clone)]
struct PluginEntryBinding {
    entry_id: String,
    plugin_entry_index: usize,
    strings_kind: StringsKind,
    string_id: Option<u32>,
}

#[instrument(skip(files), err)]
pub async fn read_localization(
    files: FileBundle,
    options: ReadOptions,
) -> Result<LocalizationBundle, PluginError> {
    let plugin_assets = files.plugins().cloned().collect::<Vec<_>>();
    if plugin_assets.len() > 1 {
        return Err(PluginError::AmbiguousPluginFiles {
            paths: plugin_assets
                .iter()
                .map(|asset| asset.path().to_string())
                .collect(),
        });
    }
    let plugin_asset =
        plugin_assets
            .into_iter()
            .next()
            .ok_or_else(|| PluginError::UnsupportedFile {
                path: "<bundle>".to_string(),
                message: "bundle does not contain a plugin file".to_string(),
            })?;
    let string_assets = files.strings().cloned().collect::<Vec<_>>();

    let plugin = task::spawn_blocking(move || {
        parse_plugin_file(&plugin_asset, ParsePluginOptions::new(options.release))
    })
    .await
    .map_err(|err| PluginError::malformed_plugin("<task>", err.to_string()))??;

    let plugin_stem = plugin_stem(plugin.path());
    let mut strings = BTreeMap::new();
    for asset in string_assets {
        let Some(info) = strings_asset_info(asset.path().as_str()) else {
            continue;
        };
        if !info.mod_name.eq_ignore_ascii_case(&plugin_stem) || info.language != options.language {
            continue;
        }
        if strings.contains_key(&info.kind) {
            return Err(PluginError::DuplicateStringsFile {
                mod_name: info.mod_name,
                language: options.language.full_name().to_string(),
                kind: info.kind.extension().to_string(),
                path: asset.path().to_string(),
            });
        }
        let parsed = parse_strings_file(&asset, options.release, options.language)?;
        strings.insert(parsed.kind(), parsed);
    }

    let mut entries = Vec::new();
    let mut bindings = Vec::new();
    let filter = PluginStringFilter::with_rules(options.extraction_filters);
    for (plugin_entry_index, plugin_entry) in plugin.entries().iter().enumerate() {
        let kind = plugin_entry.strings_kind();
        let (string_id, text) = if let Some(string_id) = plugin_entry.string_id() {
            let text = strings
                .get(&kind)
                .and_then(|file| file.get(string_id))
                .ok_or_else(|| PluginError::MissingStringId {
                    path: plugin.path().to_string(),
                    language: options.language.full_name().to_string(),
                    kind: kind.extension().to_string(),
                    id: string_id,
                })?
                .to_string();
            (Some(string_id), text)
        } else {
            (
                None,
                plugin_entry.embedded_text().unwrap_or_default().to_string(),
            )
        };
        let storage = if plugin.is_localized() {
            PluginStringStorage::Localized
        } else {
            PluginStringStorage::Embedded
        };
        let filter_input = PluginStringFilterInput {
            text: &text,
            path: plugin.path(),
            record_type: plugin_entry.record_type(),
            form_id: plugin_entry.form_id(),
            subrecord: plugin_entry.subrecord(),
            field_source: plugin_entry.source(),
            storage,
            strings_kind: kind,
            string_id,
        };
        if let Some(reason) = filter.evaluate(&filter_input) {
            trace!(
                ?reason,
                record_type = plugin_entry.record_type(),
                form_id = plugin_entry.form_id(),
                subrecord = plugin_entry.subrecord(),
                "filtered plugin string"
            );
            continue;
        }
        let entry_id = plugin_entry_id(plugin.path(), plugin_entry, plugin_entry_index, string_id);
        bindings.push(PluginEntryBinding {
            entry_id: entry_id.clone(),
            plugin_entry_index,
            strings_kind: kind,
            string_id,
        });
        entries.push(LocalizationEntry::new(StringEntry::new(
            entry_id,
            text,
            StringEntrySource::Plugin(PluginStringMetadata {
                path: plugin.path().into(),
                record_type: plugin_entry.record_type().to_string(),
                form_id: plugin_entry.form_id(),
                subrecord: plugin_entry.subrecord().to_string(),
                strings_kind: kind.extension().to_string(),
                field_source: localized_field_source_name(plugin_entry.source()).to_string(),
                storage,
                string_id,
            }),
            StringEntryContext::default(),
        )));
    }

    debug!(
        entries = entries.len(),
        strings_files = strings.len(),
        "read localization bundle"
    );
    Ok(LocalizationBundle {
        plugin,
        strings,
        entries,
        bindings,
    })
}

#[instrument(skip(bundle), err)]
pub async fn write_localization(
    mut bundle: LocalizationBundle,
    options: WriteOptions,
) -> Result<FileBundle, PluginError> {
    let mut next_id = next_available_string_id(&bundle.strings);
    for entry in bundle.entries.iter_mut().filter(|entry| entry.is_dirty()) {
        let binding = bundle
            .bindings
            .iter_mut()
            .find(|binding| binding.entry_id == entry.id())
            .expect("localization entry binding should exist");
        if bundle.plugin.is_localized() {
            if binding.string_id.is_none() {
                binding.string_id = Some(next_id);
                next_id = next_id.saturating_add(1);
            }
            let string_id = binding.string_id.expect("string id assigned");
            let file = bundle
                .strings
                .entry(binding.strings_kind)
                .or_insert_with(|| StringsFile::new(binding.strings_kind, options.language));
            file.insert(string_id, entry.text().to_string());
            bundle.plugin.entries_mut()[binding.plugin_entry_index].set_string_id(string_id);
            if let StringEntrySource::Plugin(metadata) = entry.source_mut() {
                metadata.string_id = Some(string_id);
            }
        } else {
            bundle.plugin.entries_mut()[binding.plugin_entry_index]
                .set_embedded_text(entry.text().to_string());
        }
    }

    let plugin_asset = write_plugin_file(&bundle.plugin)?;
    let mod_name = plugin_stem(plugin_asset.path().as_str());
    let mut output = vec![plugin_asset];
    for (kind, file) in bundle.strings.values().map(|file| (file.kind(), file)) {
        let path = format!(
            "Data/Strings/{}_{}.{}",
            mod_name,
            options.language.full_name(),
            kind.extension()
        );
        output.push(write_strings_file(path, file, options.release)?);
    }

    debug!(files = output.len(), "wrote localization bundle");
    Ok(FileBundle::new(output))
}

fn next_available_string_id(strings: &BTreeMap<StringsKind, StringsFile>) -> u32 {
    strings
        .values()
        .flat_map(|file| file.entries().map(|(id, _)| id))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn plugin_entry_id(
    path: &str,
    entry: &crate::PluginLocalizationEntry,
    entry_index: usize,
    string_id: Option<u32>,
) -> String {
    let suffix = string_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| format!("e{entry_index}"));
    format!(
        "plugin:{path}:{}:{:08X}:{}:{suffix}",
        entry.record_type(),
        entry.form_id(),
        entry.subrecord()
    )
}

fn localized_field_source_name(source: crate::LocalizedFieldSource) -> &'static str {
    match source {
        crate::LocalizedFieldSource::Normal => "Normal",
        crate::LocalizedFieldSource::Dl => "DL",
        crate::LocalizedFieldSource::Il => "IL",
    }
}

fn plugin_stem(path: &str) -> String {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
        .to_string()
}

struct StringsAssetInfo {
    mod_name: String,
    language: Language,
    kind: StringsKind,
}

fn strings_asset_info(path: &str) -> Option<StringsAssetInfo> {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let (stem, extension) = file_name.rsplit_once('.')?;
    let kind = match extension.to_ascii_lowercase().as_str() {
        "strings" => StringsKind::Normal,
        "dlstrings" => StringsKind::Dl,
        "ilstrings" => StringsKind::Il,
        _ => return None,
    };
    let mut languages = Language::ALL.to_vec();
    languages.sort_by_key(|language| std::cmp::Reverse(language.full_name().len()));
    for language in languages {
        let suffix = format!("_{}", language.full_name());
        if stem
            .to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
        {
            let mod_name = stem[..stem.len() - suffix.len()].to_string();
            return Some(StringsAssetInfo {
                mod_name,
                language,
                kind,
            });
        }
    }
    None
}
