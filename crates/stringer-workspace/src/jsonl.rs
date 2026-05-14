use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use stringer_core::{
    PexOperandPath, PluginStringStorage, StringEntry, StringEntryContext, StringEntrySource,
    StringEntryView,
};

use crate::WorkspaceError;
use crate::settings::{WorkspaceSettings, language_name};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct TranslationRecord {
    pub schema_version: u32,
    pub id: String,
    pub kind: String,
    pub asset_path: String,
    pub asset_language: String,
    pub source_locale: String,
    pub target_locale: String,
    pub source_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct TranslationPatchRecord {
    id: String,
    #[serde(default)]
    translated_text: Option<String>,
}

pub fn record_from_entry(
    entry: &impl StringEntryView,
    settings: &WorkspaceSettings,
) -> TranslationRecord {
    let entry = entry.string_entry();
    let (kind, asset_path, source) = source_fields(entry.source());
    TranslationRecord {
        schema_version: SCHEMA_VERSION,
        id: external_entry_id(entry),
        kind,
        asset_path,
        asset_language: language_name(settings.asset_language).to_string(),
        source_locale: settings.source_locale.clone(),
        target_locale: settings.target_locale.clone(),
        source_text: entry.text().to_string(),
        translated_text: None,
        context: context_values(entry.context()),
        source,
    }
}

pub fn write_records_jsonl(
    path: &Utf8Path,
    records: &[TranslationRecord],
) -> Result<(), WorkspaceError> {
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let file = fs::File::create(path).map_err(|source| WorkspaceError::WriteFile {
        path: path.to_owned(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record).map_err(|source| WorkspaceError::JsonLine {
            path: path.to_owned(),
            line: 0,
            source,
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| WorkspaceError::WriteFile {
                path: path.to_owned(),
                source,
            })?;
    }
    writer.flush().map_err(|source| WorkspaceError::WriteFile {
        path: path.to_owned(),
        source,
    })
}

pub fn read_translation_patches(
    path: &Utf8Path,
) -> Result<BTreeMap<String, String>, WorkspaceError> {
    let file = fs::File::open(path).map_err(|source| WorkspaceError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut patches = BTreeMap::new();
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| WorkspaceError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let patch: TranslationPatchRecord =
            serde_json::from_str(&line).map_err(|source| WorkspaceError::JsonLine {
                path: path.to_owned(),
                line: line_number,
                source,
            })?;
        let Some(translated_text) = patch.translated_text else {
            continue;
        };
        if patches.insert(patch.id.clone(), translated_text).is_some() {
            return Err(WorkspaceError::DuplicateTranslationId {
                path: path.to_owned(),
                id: patch.id,
            });
        }
    }
    Ok(patches)
}

fn source_fields(source: &StringEntrySource) -> (String, String, Option<Value>) {
    match source {
        StringEntrySource::Plugin(metadata) => (
            "plugin".to_string(),
            canonical_path(metadata.path.as_str()),
            Some(json!({
                "record_type": metadata.record_type,
                "form_id": format!("{:#010X}", metadata.form_id),
                "subrecord": metadata.subrecord,
                "strings_kind": metadata.strings_kind,
                "field_source": metadata.field_source,
                "storage": plugin_storage_name(metadata.storage),
                "string_id": metadata.string_id,
            })),
        ),
        StringEntrySource::Pex(metadata) => (
            "pex".to_string(),
            canonical_path(metadata.path.as_str()),
            Some(json!({
                "object": metadata.object,
                "state": metadata.state,
                "function": metadata.function,
                "function_kind": format!("{:?}", metadata.function_kind),
                "instruction_index": metadata.instruction_index,
                "opcode": metadata.opcode,
                "operand": pex_operand_name(metadata.operand),
                "string_id": metadata.string_id,
            })),
        ),
        StringEntrySource::Scaleform(metadata) => (
            "scaleform".to_string(),
            canonical_path(metadata.path.as_str()),
            Some(json!({
                "key": metadata.key,
            })),
        ),
    }
}

fn context_values(context: &StringEntryContext) -> BTreeMap<String, String> {
    context.values().clone()
}

fn plugin_storage_name(storage: PluginStringStorage) -> &'static str {
    match storage {
        PluginStringStorage::Localized => "localized",
        PluginStringStorage::Embedded => "embedded",
    }
}

fn pex_operand_name(operand: PexOperandPath) -> String {
    match operand {
        PexOperandPath::Fixed(index) => format!("fixed-{index}"),
        PexOperandPath::Variadic(index) => format!("variadic-{index}"),
    }
}

pub(crate) fn external_entry_id(entry: &StringEntry) -> String {
    match entry.source() {
        StringEntrySource::Plugin(metadata) => {
            replace_path_segment(entry.id(), "plugin:", metadata.path.as_str())
        }
        StringEntrySource::Pex(metadata) => {
            replace_path_segment(entry.id(), "pex:", metadata.path.as_str())
        }
        StringEntrySource::Scaleform(metadata) => {
            replace_path_segment(entry.id(), "scaleform:", metadata.path.as_str())
        }
    }
}

fn replace_path_segment(raw_id: &str, prefix: &str, path: &str) -> String {
    let Some(rest) = raw_id.strip_prefix(prefix) else {
        return raw_id.to_string();
    };
    for candidate in [
        path.to_string(),
        path.replace('/', "\\"),
        path.replace('\\', "/"),
    ] {
        if let Some(tail) = rest.strip_prefix(&candidate) {
            return format!("{prefix}{}{tail}", canonical_path(path));
        }
    }
    raw_id.to_string()
}

fn canonical_path(value: &str) -> String {
    value.replace('\\', "/")
}
