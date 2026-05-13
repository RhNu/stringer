#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::str;

use bytes::{Bytes, BytesMut};
use camino::{Utf8Path, Utf8PathBuf};
use stringer_core::{
    FileAsset, FileBundle, FileRole, Language, ScaleformStringMetadata, StringEntry,
    StringEntryBundle, StringEntryContext, StringEntrySource,
};
use thiserror::Error;
use tracing::{debug, instrument, trace, warn};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScaleformError {
    #[error("unsupported file `{path}`: {message}")]
    UnsupportedFile { path: String, message: String },

    #[error("invalid encoding in `{path}`: {message}")]
    InvalidEncoding { path: String, message: String },

    #[error("malformed row in `{path}` at line {line}: {message}")]
    MalformedRow {
        path: String,
        line: usize,
        message: String,
    },

    #[error("duplicate scaleform key `{key}` in `{path}` at line {line}")]
    DuplicateKey {
        path: String,
        line: usize,
        key: String,
    },

    #[error("invalid scaleform key `{key}`: {message}")]
    InvalidKey { key: String, message: String },

    #[error("invalid scaleform text for key `{key}`: {message}")]
    InvalidText { key: String, message: String },

    #[error("string entry `{entry_id}` no longer resolves to a scaleform key")]
    InvalidStringEntryBinding { entry_id: String },
}

impl ScaleformError {
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::MalformedRow { line, .. } | Self::DuplicateKey { line, .. } => Some(*line),
            _ => None,
        }
    }

    fn unsupported_file(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::UnsupportedFile {
            path: path.into(),
            message: message.into(),
        }
    }

    fn invalid_key(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidKey {
            key: key.into(),
            message: message.into(),
        }
    }

    fn invalid_text(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidText {
            key: key.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScaleformTranslationFile {
    path: Utf8PathBuf,
    lines: Vec<ScaleformLine>,
    entries: Vec<ScaleformEntry>,
    original: Option<FileAsset>,
    newline: &'static str,
    trailing_newline: bool,
    dirty: bool,
}

impl ScaleformTranslationFile {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            lines: Vec::new(),
            entries: Vec::new(),
            original: None,
            newline: "\r\n",
            trailing_newline: true,
            dirty: true,
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn entries(&self) -> &[ScaleformEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [ScaleformEntry] {
        &mut self.entries
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.text.as_str())
    }

    pub fn entry_mut(&mut self, key: &str) -> Option<&mut ScaleformEntry> {
        self.entries.iter_mut().find(|entry| entry.key == key)
    }

    pub fn insert(
        &mut self,
        key: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Option<String>, ScaleformError> {
        let key = key.into();
        validate_key(&key)?;
        let text = text.into();
        validate_text(&key, &text)?;
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            let old = std::mem::replace(&mut entry.text, text);
            entry.dirty = true;
            self.dirty = true;
            return Ok(Some(old));
        }

        self.lines.push(ScaleformLine::Entry { key: key.clone() });
        self.entries.push(ScaleformEntry {
            key,
            text,
            dirty: true,
        });
        self.dirty = true;
        Ok(None)
    }

    fn is_dirty(&self) -> bool {
        self.dirty || self.entries.iter().any(ScaleformEntry::is_dirty)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleformEntry {
    key: String,
    text: String,
    dirty: bool,
}

impl ScaleformEntry {
    pub fn key(&self) -> &str {
        &self.key
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

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScaleformLine {
    Raw(String),
    Entry { key: String },
}

#[derive(Debug, Clone)]
pub struct ScaleformTranslationBundle {
    files: BTreeMap<String, ScaleformTranslationFile>,
    entries: Vec<StringEntry>,
    bindings: Vec<ScaleformEntryBinding>,
}

impl ScaleformTranslationBundle {
    pub fn entries(&self) -> &[StringEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [StringEntry] {
        &mut self.entries
    }
}

impl StringEntryBundle for ScaleformTranslationBundle {
    type Entry = StringEntry;

    fn string_entries(&self) -> &[StringEntry] {
        &self.entries
    }

    fn string_entries_mut(&mut self) -> &mut [StringEntry] {
        &mut self.entries
    }
}

#[derive(Debug, Clone)]
struct ScaleformEntryBinding {
    entry_id: String,
    path: String,
    key: String,
}

#[instrument(skip(asset), fields(path = %asset.path()), err)]
pub fn parse_scaleform_translation_file(
    asset: &FileAsset,
) -> Result<ScaleformTranslationFile, ScaleformError> {
    if asset.role() != FileRole::Scaleform {
        warn!(role = ?asset.role(), "unsupported scaleform input role");
        return Err(ScaleformError::unsupported_file(
            asset.path().to_string(),
            "expected Data/Interface/Translations/*.txt",
        ));
    }

    let text = decode_text(asset)?;
    let newline = dominant_newline(&text);
    let (rows, trailing_newline) = split_rows(&text);
    let mut seen = HashSet::new();
    let mut lines = Vec::with_capacity(rows.len());
    let mut entries = Vec::new();

    for (line, line_number) in rows {
        if line.trim().is_empty()
            || line.trim_start().starts_with('#')
            || line.trim_start().starts_with(';')
        {
            lines.push(ScaleformLine::Raw(line));
            continue;
        }
        let Some((key, value)) = line.split_once('\t') else {
            return Err(ScaleformError::MalformedRow {
                path: asset.path().to_string(),
                line: line_number,
                message: "expected a tab between key and value".to_string(),
            });
        };
        if !key.starts_with('$') || key.len() == 1 {
            return Err(ScaleformError::MalformedRow {
                path: asset.path().to_string(),
                line: line_number,
                message: "key must start with `$`".to_string(),
            });
        }
        if let Err(error) = validate_key(key) {
            let ScaleformError::InvalidKey { message, .. } = error else {
                unreachable!("validate_key only returns invalid key errors");
            };
            return Err(ScaleformError::MalformedRow {
                path: asset.path().to_string(),
                line: line_number,
                message,
            });
        }
        if !seen.insert(key.to_string()) {
            return Err(ScaleformError::DuplicateKey {
                path: asset.path().to_string(),
                line: line_number,
                key: key.to_string(),
            });
        }
        trace!(key, line = line_number, "parsed scaleform entry");
        lines.push(ScaleformLine::Entry {
            key: key.to_string(),
        });
        entries.push(ScaleformEntry {
            key: key.to_string(),
            text: value.to_string(),
            dirty: false,
        });
    }

    debug!(entries = entries.len(), "parsed scaleform translation file");
    Ok(ScaleformTranslationFile {
        path: asset.path().to_owned(),
        lines,
        entries,
        original: Some(asset.clone()),
        newline,
        trailing_newline,
        dirty: false,
    })
}

#[instrument(skip(file), fields(path = %file.path()), err)]
pub fn write_scaleform_translation_file(
    file: &ScaleformTranslationFile,
) -> Result<FileAsset, ScaleformError> {
    if !file.is_dirty()
        && let Some(original) = &file.original
    {
        trace!("preserving unmodified scaleform bytes");
        return Ok(original.clone());
    }

    let entries = file
        .entries
        .iter()
        .map(|entry| (entry.key.as_str(), entry.text.as_str()))
        .collect::<HashMap<_, _>>();
    let mut text = String::new();
    for (index, line) in file.lines.iter().enumerate() {
        match line {
            ScaleformLine::Raw(raw) => text.push_str(raw),
            ScaleformLine::Entry { key } => {
                let value =
                    entries
                        .get(key.as_str())
                        .ok_or_else(|| ScaleformError::MalformedRow {
                            path: file.path().to_string(),
                            line: index + 1,
                            message: format!("key `{key}` has no entry"),
                        })?;
                validate_text(key, value)?;
                text.push_str(key);
                text.push('\t');
                text.push_str(value);
            }
        }
        if index + 1 < file.lines.len() || file.trailing_newline {
            text.push_str(file.newline);
        }
    }

    let bytes = encode_utf16le_bom(&text);
    debug!(
        entries = file.entries.len(),
        bytes = bytes.len(),
        "wrote scaleform translation file"
    );
    Ok(FileAsset::new(file.path().to_owned(), bytes))
}

#[instrument(skip(files), fields(language = %language.full_name()), err)]
pub fn read_scaleform_translations(
    files: FileBundle,
    language: Language,
) -> Result<ScaleformTranslationBundle, ScaleformError> {
    let mut parsed_files = BTreeMap::new();
    let mut entries = Vec::new();
    let mut bindings = Vec::new();

    for asset in files.scaleform() {
        let Some(info) = scaleform_asset_info(asset.path().as_str()) else {
            trace!(path = %asset.path(), "skipping unmatched scaleform path");
            continue;
        };
        if info.language != language {
            trace!(
                path = %asset.path(),
                asset_language = %info.language.full_name(),
                "skipping scaleform file for another language"
            );
            continue;
        }
        let file = parse_scaleform_translation_file(asset)?;
        for entry in file.entries() {
            let entry_id = scaleform_entry_id(file.path(), entry.key());
            entries.push(StringEntry::new(
                entry_id.clone(),
                entry.text(),
                StringEntrySource::Scaleform(ScaleformStringMetadata {
                    path: file.path().to_owned(),
                    key: Some(entry.key().to_string()),
                }),
                StringEntryContext::default(),
            ));
            bindings.push(ScaleformEntryBinding {
                entry_id,
                path: file.path().to_string(),
                key: entry.key().to_string(),
            });
        }
        parsed_files.insert(file.path().to_string(), file);
    }

    debug!(
        entries = entries.len(),
        files = parsed_files.len(),
        "read scaleform translation bundle"
    );
    Ok(ScaleformTranslationBundle {
        files: parsed_files,
        entries,
        bindings,
    })
}

#[instrument(skip(bundle), err)]
pub fn write_scaleform_translations(
    mut bundle: ScaleformTranslationBundle,
) -> Result<FileBundle, ScaleformError> {
    let dirty_entries = bundle
        .entries
        .iter()
        .filter(|entry| entry.is_dirty())
        .count();
    for entry in bundle.entries.iter_mut().filter(|entry| entry.is_dirty()) {
        let binding = bundle
            .bindings
            .iter()
            .find(|binding| binding.entry_id == entry.id())
            .cloned()
            .ok_or_else(|| ScaleformError::InvalidStringEntryBinding {
                entry_id: entry.id().to_string(),
            })?;
        let file = bundle.files.get_mut(&binding.path).ok_or_else(|| {
            ScaleformError::InvalidStringEntryBinding {
                entry_id: binding.entry_id.clone(),
            }
        })?;
        let scaleform_entry = file.entry_mut(&binding.key).ok_or_else(|| {
            ScaleformError::InvalidStringEntryBinding {
                entry_id: binding.entry_id.clone(),
            }
        })?;
        scaleform_entry.set_text(entry.text().to_string());
        if let StringEntrySource::Scaleform(metadata) = entry.source_mut() {
            metadata.key = Some(binding.key);
        }
    }

    let mut output = Vec::with_capacity(bundle.files.len());
    for file in bundle.files.values() {
        output.push(write_scaleform_translation_file(file)?);
    }
    debug!(
        dirty_entries,
        files = output.len(),
        "wrote scaleform translation bundle"
    );
    Ok(FileBundle::new(output))
}

fn decode_text(asset: &FileAsset) -> Result<String, ScaleformError> {
    let bytes = asset.bytes();
    if bytes.starts_with(&[0xFF, 0xFE]) {
        if !(bytes.len() - 2).is_multiple_of(2) {
            return Err(ScaleformError::InvalidEncoding {
                path: asset.path().to_string(),
                message: "UTF-16 LE data has an odd byte length".to_string(),
            });
        }
        let units = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|err| ScaleformError::InvalidEncoding {
            path: asset.path().to_string(),
            message: err.to_string(),
        });
    }

    let utf8 = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .unwrap_or(bytes.as_ref());
    str::from_utf8(utf8)
        .map(str::to_string)
        .map_err(|err| ScaleformError::InvalidEncoding {
            path: asset.path().to_string(),
            message: err.to_string(),
        })
}

fn encode_utf16le_bom(text: &str) -> Bytes {
    let mut bytes = BytesMut::from(&[0xFF, 0xFE][..]);
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.freeze()
}

fn split_rows(text: &str) -> (Vec<(String, usize)>, bool) {
    let mut rows = Vec::new();
    let mut start = 0;
    while let Some(relative) = text[start..].find('\n') {
        let end = start + relative;
        let mut line = &text[start..end];
        if let Some(stripped) = line.strip_suffix('\r') {
            line = stripped;
        }
        rows.push((line.to_string(), rows.len() + 1));
        start = end + 1;
    }
    if start < text.len() {
        rows.push((text[start..].to_string(), rows.len() + 1));
    }
    (rows, text.ends_with('\n'))
}

fn dominant_newline(text: &str) -> &'static str {
    let crlf = text.matches("\r\n").count();
    let lf = text.bytes().filter(|byte| *byte == b'\n').count();
    let lone_lf = lf.saturating_sub(crlf);
    if crlf >= lone_lf { "\r\n" } else { "\n" }
}

fn validate_key(key: &str) -> Result<(), ScaleformError> {
    if !key.starts_with('$') || key.len() == 1 {
        return Err(ScaleformError::invalid_key(key, "key must start with `$`"));
    }
    if key.contains(['\t', '\r', '\n']) {
        return Err(ScaleformError::invalid_key(
            key,
            "key must not contain tabs or newlines",
        ));
    }
    Ok(())
}

struct ScaleformAssetInfo {
    language: Language,
}

fn scaleform_asset_info(path: &str) -> Option<ScaleformAssetInfo> {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let (stem, extension) = file_name.rsplit_once('.')?;
    if !extension.eq_ignore_ascii_case("txt") {
        return None;
    }
    for language in Language::ALL {
        let suffix = format!("_{}", language.full_name());
        if stem
            .to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
        {
            return Some(ScaleformAssetInfo { language });
        }
    }
    None
}

fn scaleform_entry_id(path: &Utf8Path, key: &str) -> String {
    format!("scaleform:{path}:{key}")
}

fn validate_text(key: &str, text: &str) -> Result<(), ScaleformError> {
    if text.contains(['\r', '\n']) {
        return Err(ScaleformError::invalid_text(
            key,
            "text must not contain newlines",
        ));
    }
    Ok(())
}
