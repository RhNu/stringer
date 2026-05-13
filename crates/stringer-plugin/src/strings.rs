use std::collections::BTreeMap;

use bytes::Bytes;
use stringer_core::{FileAsset, FileFormat};
use tracing::{debug, instrument};

use crate::encoding::{decode_text, encode_text};
use crate::{GameRelease, Language, PluginError, StringsKind};

#[derive(Debug, Clone)]
pub struct StringsFile {
    kind: StringsKind,
    language: Language,
    entries: BTreeMap<u32, String>,
    original: Option<FileAsset>,
    dirty: bool,
}

impl StringsFile {
    pub fn new(kind: StringsKind, language: Language) -> Self {
        Self {
            kind,
            language,
            entries: BTreeMap::new(),
            original: None,
            dirty: true,
        }
    }

    pub fn kind(&self) -> StringsKind {
        self.kind
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn insert(&mut self, id: u32, text: impl Into<String>) -> Option<String> {
        self.dirty = true;
        self.entries.insert(id, text.into())
    }

    pub fn get(&self, id: u32) -> Option<&str> {
        self.entries.get(&id).map(String::as_str)
    }

    pub fn entries(&self) -> impl Iterator<Item = (u32, &str)> {
        self.entries.iter().map(|(id, text)| (*id, text.as_str()))
    }

    pub(crate) fn set_clean_original(&mut self, asset: FileAsset) {
        self.original = Some(asset);
        self.dirty = false;
    }

    pub(crate) fn original(&self) -> Option<&FileAsset> {
        self.original.as_ref()
    }

    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[instrument(skip(asset), fields(path = %asset.path()))]
pub fn parse_strings_file(
    asset: &FileAsset,
    release: GameRelease,
    language: Language,
) -> Result<StringsFile, PluginError> {
    let kind = kind_from_asset(asset)?;
    let path = asset.path().to_string();
    let bytes = asset.bytes().as_ref();
    if bytes.len() < 8 {
        return Err(PluginError::malformed_strings(
            path,
            "file is shorter than the 8-byte strings header",
        ));
    }

    let count = read_u32(bytes, 0) as usize;
    let data_size = read_u32(bytes, 4) as usize;
    let directory_len = count.checked_mul(8).ok_or_else(|| {
        PluginError::malformed_strings(asset.path().to_string(), "directory length overflow")
    })?;
    let data_start = 8usize.checked_add(directory_len).ok_or_else(|| {
        PluginError::malformed_strings(asset.path().to_string(), "data start overflow")
    })?;
    let data_end = data_start.checked_add(data_size).ok_or_else(|| {
        PluginError::malformed_strings(asset.path().to_string(), "data size overflow")
    })?;
    if bytes.len() < data_end {
        return Err(PluginError::malformed_strings(
            path,
            "declared string data section extends past end of file",
        ));
    }

    let data = &bytes[data_start..data_end];
    let mut strings = StringsFile {
        kind,
        language,
        entries: BTreeMap::new(),
        original: None,
        dirty: false,
    };

    for index in 0..count {
        let directory_offset = 8 + index * 8;
        let id = read_u32(bytes, directory_offset);
        let offset = read_u32(bytes, directory_offset + 4) as usize;
        if strings.entries.contains_key(&id) {
            return Err(PluginError::DuplicateStringId {
                path: asset.path().to_string(),
                id,
            });
        }
        let raw = extract_raw_string(asset.path().as_str(), kind, data, offset)?;
        let text = decode_text(release, language, raw)?;
        strings.entries.insert(id, text);
    }

    debug!(
        count = strings.entries.len(),
        ?kind,
        ?language,
        "parsed strings file"
    );
    strings.set_clean_original(asset.clone());
    Ok(strings)
}

#[instrument(skip(file), fields(path = %path.as_ref()))]
pub fn write_strings_file(
    path: impl AsRef<str>,
    file: &StringsFile,
    release: GameRelease,
) -> Result<FileAsset, PluginError> {
    if !file.is_dirty()
        && let Some(original) = file.original()
        && original.path().as_str().eq_ignore_ascii_case(path.as_ref())
    {
        return Ok(original.clone());
    }

    let mut directory = Vec::with_capacity(file.entries.len() * 8);
    let mut data = Vec::new();
    for (id, text) in file.entries() {
        let offset = u32::try_from(data.len()).map_err(|_| {
            PluginError::malformed_strings(path.as_ref(), "strings data exceeds u32::MAX")
        })?;
        directory.extend_from_slice(&id.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        let mut encoded = encode_text(release, file.language, text)?;
        match file.kind {
            StringsKind::Normal => {
                data.append(&mut encoded);
                data.push(0);
            }
            StringsKind::Dl | StringsKind::Il => {
                let len = u32::try_from(encoded.len() + 1).map_err(|_| {
                    PluginError::malformed_strings(path.as_ref(), "single string exceeds u32::MAX")
                })?;
                data.extend_from_slice(&len.to_le_bytes());
                data.append(&mut encoded);
                data.push(0);
            }
        }
    }

    let mut bytes = Vec::with_capacity(8 + directory.len() + data.len());
    let count = u32::try_from(file.entries.len())
        .map_err(|_| PluginError::malformed_strings(path.as_ref(), "too many strings"))?;
    let data_len = u32::try_from(data.len())
        .map_err(|_| PluginError::malformed_strings(path.as_ref(), "strings data too large"))?;
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend(directory);
    bytes.extend(data);

    debug!(count, ?file.kind, ?file.language, "wrote strings file");
    Ok(FileAsset::new(path.as_ref(), Bytes::from(bytes)))
}

fn kind_from_asset(asset: &FileAsset) -> Result<StringsKind, PluginError> {
    match asset.format() {
        FileFormat::Strings => Ok(StringsKind::Normal),
        FileFormat::DlStrings => Ok(StringsKind::Dl),
        FileFormat::IlStrings => Ok(StringsKind::Il),
        _ => Err(PluginError::UnsupportedFile {
            path: asset.path().to_string(),
            message: "expected .STRINGS, .DLSTRINGS, or .ILSTRINGS".to_string(),
        }),
    }
}

fn extract_raw_string<'a>(
    path: &str,
    kind: StringsKind,
    data: &'a [u8],
    offset: usize,
) -> Result<&'a [u8], PluginError> {
    if offset >= data.len() {
        return Err(PluginError::malformed_strings(
            path,
            "string directory offset points outside the data section",
        ));
    }
    match kind {
        StringsKind::Normal => {
            let relative_end = data[offset..]
                .iter()
                .position(|byte| *byte == 0)
                .ok_or_else(|| {
                    PluginError::malformed_strings(
                        path,
                        "null terminator missing from normal string",
                    )
                })?;
            Ok(&data[offset..offset + relative_end])
        }
        StringsKind::Dl | StringsKind::Il => {
            if data.len() < offset + 4 {
                return Err(PluginError::malformed_strings(
                    path,
                    "length-prefixed string is missing its length",
                ));
            }
            let len = read_u32(data, offset) as usize;
            let start = offset + 4;
            let end = start.checked_add(len).ok_or_else(|| {
                PluginError::malformed_strings(path, "length-prefixed string length overflow")
            })?;
            if end > data.len() {
                return Err(PluginError::malformed_strings(
                    path,
                    "length-prefixed string extends past the data section",
                ));
            }
            let bytes = &data[start..end];
            Ok(bytes.strip_suffix(&[0]).unwrap_or(bytes))
        }
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 slice"))
}
