use std::collections::BTreeMap;

use stringer_core::FileBundle;
use tokio::task;
use tracing::{debug, instrument};

use crate::{
    GameRelease, Language, ParsePluginOptions, ParsedPlugin, PluginError, StringsFile, StringsKind,
    parse_plugin_file, parse_strings_file, write_plugin_file, write_strings_file,
};

#[derive(Debug, Clone, Copy)]
pub struct ReadOptions {
    release: GameRelease,
    language: Language,
}

impl ReadOptions {
    pub fn new(release: GameRelease, language: Language) -> Self {
        Self { release, language }
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
}

impl LocalizationBundle {
    pub fn entries(&self) -> &[LocalizationEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [LocalizationEntry] {
        &mut self.entries
    }
}

#[derive(Debug, Clone)]
pub struct LocalizationEntry {
    plugin_entry_index: usize,
    strings_kind: StringsKind,
    string_id: Option<u32>,
    text: String,
    dirty: bool,
}

impl LocalizationEntry {
    pub fn strings_kind(&self) -> StringsKind {
        self.strings_kind
    }

    pub fn string_id(&self) -> Option<u32> {
        self.string_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.text != text {
            self.text = text;
            self.dirty = true;
        }
    }
}

#[instrument(skip(files))]
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
        entries.push(LocalizationEntry {
            plugin_entry_index,
            strings_kind: kind,
            string_id,
            text,
            dirty: false,
        });
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
    })
}

#[instrument(skip(bundle))]
pub async fn write_localization(
    mut bundle: LocalizationBundle,
    options: WriteOptions,
) -> Result<FileBundle, PluginError> {
    let mut next_id = next_available_string_id(&bundle.strings);
    for entry in bundle.entries.iter_mut().filter(|entry| entry.dirty) {
        if bundle.plugin.is_localized() {
            if entry.string_id.is_none() {
                entry.string_id = Some(next_id);
                next_id = next_id.saturating_add(1);
            }
            let string_id = entry.string_id.expect("string id assigned");
            let file = bundle
                .strings
                .entry(entry.strings_kind)
                .or_insert_with(|| StringsFile::new(entry.strings_kind, options.language));
            file.insert(string_id, entry.text.clone());
            bundle.plugin.entries_mut()[entry.plugin_entry_index].set_string_id(string_id);
        } else {
            bundle.plugin.entries_mut()[entry.plugin_entry_index]
                .set_embedded_text(entry.text.clone());
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
    let mut languages = all_languages();
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

fn all_languages() -> Vec<Language> {
    vec![
        Language::English,
        Language::German,
        Language::Italian,
        Language::Spanish,
        Language::SpanishMexico,
        Language::French,
        Language::Polish,
        Language::PortugueseBrazil,
        Language::Chinese,
        Language::Russian,
        Language::Japanese,
        Language::Czech,
        Language::Hungarian,
        Language::Danish,
        Language::Finnish,
        Language::Greek,
        Language::Norwegian,
        Language::Swedish,
        Language::Turkish,
        Language::Arabic,
        Language::Korean,
        Language::Thai,
        Language::ChineseSimplified,
    ]
}
