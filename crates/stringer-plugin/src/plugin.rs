use std::collections::HashSet;
use std::io::{Read, Write};

use bytes::Bytes;
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use stringer_core::{FileAsset, FileRole};
use tracing::{debug, instrument};

use crate::encoding::{decode_text, encode_text};
use crate::registry::find_localized_field;
use crate::{GameRelease, Language, LocalizedFieldSource, PluginError, StringsKind};

const MAJOR_HEADER_LEN: usize = 24;
const GROUP_HEADER_LEN: usize = 24;
const SUB_HEADER_LEN: usize = 6;
const COMPRESSED_FLAG: u32 = 0x0004_0000;
const LOCALIZED_FLAG: u32 = 0x0000_0080;
const MAX_DECOMPRESSED_RECORD_LEN: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct ParsePluginOptions {
    release: GameRelease,
    embedded_language: Language,
}

impl ParsePluginOptions {
    pub fn new(release: GameRelease) -> Self {
        Self {
            release,
            embedded_language: Language::English,
        }
    }

    pub fn with_embedded_language(mut self, language: Language) -> Self {
        self.embedded_language = language;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ParsedPlugin {
    original: FileAsset,
    localized: bool,
    nodes: Vec<PluginNode>,
    records: Vec<PluginRecord>,
    entries: Vec<PluginLocalizationEntry>,
    options: ParsePluginOptions,
}

impl ParsedPlugin {
    pub fn is_localized(&self) -> bool {
        self.localized
    }

    pub fn entries(&self) -> &[PluginLocalizationEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [PluginLocalizationEntry] {
        &mut self.entries
    }

    pub fn records(&self) -> &[PluginRecord] {
        &self.records
    }

    pub fn path(&self) -> &str {
        self.original.path().as_str()
    }

    pub(crate) fn original(&self) -> &FileAsset {
        &self.original
    }
}

#[derive(Debug, Clone)]
enum PluginNode {
    Raw(Vec<u8>),
    Group(PluginGroup),
    Record(usize),
}

#[derive(Debug, Clone)]
struct PluginGroup {
    header: Vec<u8>,
    original: Vec<u8>,
    children: Vec<PluginNode>,
}

#[derive(Debug, Clone)]
pub struct PluginRecord {
    record_type: String,
    form_id: u32,
    flags: u32,
    header: Vec<u8>,
    original: Vec<u8>,
    subrecords: Vec<PluginSubrecord>,
}

impl PluginRecord {
    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn form_id(&self) -> u32 {
        self.form_id
    }

    pub fn is_compressed(&self) -> bool {
        self.flags & COMPRESSED_FLAG != 0
    }
}

#[derive(Debug, Clone)]
struct PluginSubrecord {
    record_type: String,
    original: Vec<u8>,
    content: Vec<u8>,
    overflow: bool,
    entry_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PluginLocalizationEntry {
    record_index: usize,
    record_type: String,
    form_id: u32,
    subrecord: String,
    source: LocalizedFieldSource,
    string_id: Option<u32>,
    embedded_text: Option<String>,
    dirty: bool,
}

impl PluginLocalizationEntry {
    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn form_id(&self) -> u32 {
        self.form_id
    }

    pub fn subrecord(&self) -> &str {
        &self.subrecord
    }

    pub fn source(&self) -> LocalizedFieldSource {
        self.source
    }

    pub fn strings_kind(&self) -> StringsKind {
        self.source.strings_kind()
    }

    pub fn string_id(&self) -> Option<u32> {
        self.string_id
    }

    pub fn set_string_id(&mut self, id: u32) {
        if self.string_id != Some(id) {
            self.string_id = Some(id);
            self.embedded_text = None;
            self.dirty = true;
        }
    }

    pub fn embedded_text(&self) -> Option<&str> {
        self.embedded_text.as_deref()
    }

    pub fn set_embedded_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.embedded_text.as_deref() != Some(text.as_str()) {
            self.embedded_text = Some(text);
            self.string_id = None;
            self.dirty = true;
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[instrument(skip(asset), fields(path = %asset.path()), err)]
pub fn parse_plugin_file(
    asset: &FileAsset,
    options: ParsePluginOptions,
) -> Result<ParsedPlugin, PluginError> {
    if asset.role() != FileRole::Plugin {
        return Err(PluginError::UnsupportedFile {
            path: asset.path().to_string(),
            message: "expected .esp, .esm, or .esl plugin".to_string(),
        });
    }
    let mut parser = PluginParser {
        path: asset.path().to_string(),
        bytes: asset.bytes().as_ref(),
        options,
        localized: false,
        records: Vec::new(),
        entries: Vec::new(),
    };
    let nodes = parser.parse_nodes(0, parser.bytes.len())?;
    debug!(
        records = parser.records.len(),
        entries = parser.entries.len(),
        localized = parser.localized,
        "parsed plugin file"
    );
    Ok(ParsedPlugin {
        original: asset.clone(),
        localized: parser.localized,
        nodes,
        records: parser.records,
        entries: parser.entries,
        options,
    })
}

#[instrument(skip(plugin), fields(path = plugin.path()), err)]
pub fn write_plugin_file(plugin: &ParsedPlugin) -> Result<FileAsset, PluginError> {
    if !plugin.entries.iter().any(PluginLocalizationEntry::is_dirty) {
        debug!("preserving unmodified plugin bytes");
        return Ok(plugin.original().clone());
    }

    let dirty_records = plugin
        .entries
        .iter()
        .filter(|entry| entry.is_dirty())
        .map(|entry| entry.record_index)
        .collect::<HashSet<_>>();
    let mut bytes = Vec::new();
    for node in &plugin.nodes {
        write_node(node, plugin, &dirty_records, &mut bytes)?;
    }
    debug!(
        records = dirty_records.len(),
        entries = plugin
            .entries
            .iter()
            .filter(|entry| entry.is_dirty())
            .count(),
        "wrote plugin file"
    );
    Ok(FileAsset::new(plugin.path(), Bytes::from(bytes)))
}

struct PluginParser<'a> {
    path: String,
    bytes: &'a [u8],
    options: ParsePluginOptions,
    localized: bool,
    records: Vec<PluginRecord>,
    entries: Vec<PluginLocalizationEntry>,
}

impl PluginParser<'_> {
    fn parse_nodes(&mut self, start: usize, end: usize) -> Result<Vec<PluginNode>, PluginError> {
        let mut nodes = Vec::new();
        let mut offset = start;
        while offset < end {
            if end - offset < 4 {
                return Err(self.malformed("trailing bytes are too short for a record type"));
            }
            let record_type = fourcc(&self.bytes[offset..offset + 4]);
            if record_type == "GRUP" {
                let (group, next) = self.parse_group(offset, end)?;
                nodes.push(PluginNode::Group(group));
                offset = next;
            } else if end - offset >= MAJOR_HEADER_LEN {
                let next = self.parse_major(offset, end, &record_type, &mut nodes)?;
                offset = next;
            } else {
                return Err(self.malformed("truncated major record header"));
            }
        }
        Ok(nodes)
    }

    fn parse_group(
        &mut self,
        offset: usize,
        parent_end: usize,
    ) -> Result<(PluginGroup, usize), PluginError> {
        if parent_end - offset < GROUP_HEADER_LEN {
            return Err(self.malformed("truncated group header"));
        }
        let group_len = read_u32(self.bytes, offset + 4) as usize;
        if group_len < GROUP_HEADER_LEN {
            return Err(self.malformed("group length is shorter than its header"));
        }
        let end = offset
            .checked_add(group_len)
            .ok_or_else(|| self.malformed("group length overflow"))?;
        if end > parent_end || end > self.bytes.len() {
            return Err(self.malformed("group extends past parent boundary"));
        }
        let children = self.parse_nodes(offset + GROUP_HEADER_LEN, end)?;
        Ok((
            PluginGroup {
                header: self.bytes[offset..offset + GROUP_HEADER_LEN].to_vec(),
                original: self.bytes[offset..end].to_vec(),
                children,
            },
            end,
        ))
    }

    fn parse_major(
        &mut self,
        offset: usize,
        parent_end: usize,
        record_type: &str,
        nodes: &mut Vec<PluginNode>,
    ) -> Result<usize, PluginError> {
        let content_len = read_u32(self.bytes, offset + 4) as usize;
        let end = offset
            .checked_add(MAJOR_HEADER_LEN)
            .and_then(|header_end| header_end.checked_add(content_len))
            .ok_or_else(|| self.malformed("major record length overflow"))?;
        if end > parent_end || end > self.bytes.len() {
            return Err(self.malformed("major record extends past parent boundary"));
        }

        let flags = read_u32(self.bytes, offset + 8);
        let form_id = read_u32(self.bytes, offset + 12);
        if record_type == "TES4" {
            self.localized = flags & LOCALIZED_FLAG != 0;
            nodes.push(PluginNode::Raw(self.bytes[offset..end].to_vec()));
            return Ok(end);
        }

        let content = &self.bytes[offset + MAJOR_HEADER_LEN..end];
        let subrecord_content = if flags & COMPRESSED_FLAG != 0 {
            decompress_record_content(content, content_len, &self.path)?
        } else {
            content.to_vec()
        };
        let record_index = self.records.len();
        let mut subrecords = parse_subrecords(&self.path, &subrecord_content)?;
        for subrecord_index in 0..subrecords.len() {
            let Some(source) = localized_source_for(record_type, &subrecords, subrecord_index)
            else {
                continue;
            };
            let subrecord = &mut subrecords[subrecord_index];
            let (string_id, embedded_text) = if self.localized {
                if subrecord.content.len() != 4 {
                    return Err(self.malformed(format!(
                        "localized field {record_type}.{} should contain a 4-byte strings id",
                        subrecord.record_type
                    )));
                }
                (Some(read_u32(&subrecord.content, 0)), None)
            } else {
                let raw = strip_optional_null(&subrecord.content);
                let text = decode_text(self.options.release, self.options.embedded_language, raw)?;
                (None, Some(text))
            };
            let entry_index = self.entries.len();
            subrecord.entry_index = Some(entry_index);
            self.entries.push(PluginLocalizationEntry {
                record_index,
                record_type: record_type.to_string(),
                form_id,
                subrecord: subrecord.record_type.clone(),
                source,
                string_id,
                embedded_text,
                dirty: false,
            });
        }

        self.records.push(PluginRecord {
            record_type: record_type.to_string(),
            form_id,
            flags,
            header: self.bytes[offset..offset + MAJOR_HEADER_LEN].to_vec(),
            original: self.bytes[offset..end].to_vec(),
            subrecords,
        });
        nodes.push(PluginNode::Record(record_index));
        Ok(end)
    }

    fn malformed(&self, message: impl Into<String>) -> PluginError {
        PluginError::malformed_plugin(self.path.clone(), message)
    }
}

fn localized_source_for(
    record_type: &str,
    subrecords: &[PluginSubrecord],
    index: usize,
) -> Option<LocalizedFieldSource> {
    let subrecord = &subrecords[index].record_type;
    if record_type == "PERK" && subrecord == "EPFT" {
        return None;
    }
    if record_type == "GMST" && subrecord == "DATA" && !is_string_game_setting(subrecords) {
        return None;
    }
    if record_type == "QUST" && subrecord == "NNAM" {
        return Some(if is_quest_objective_display_text(subrecords, index) {
            LocalizedFieldSource::Normal
        } else {
            LocalizedFieldSource::Dl
        });
    }
    if record_type == "FACT" && subrecord == "FNAM" {
        return Some(LocalizedFieldSource::Normal);
    }
    find_localized_field(record_type, subrecord).map(|field| field.source)
}

fn is_string_game_setting(subrecords: &[PluginSubrecord]) -> bool {
    subrecords
        .iter()
        .find(|subrecord| subrecord.record_type == "EDID")
        .and_then(|subrecord| subrecord.content.first())
        .is_some_and(|first| matches!(*first, b's' | b'S'))
}

fn is_quest_objective_display_text(subrecords: &[PluginSubrecord], index: usize) -> bool {
    let mut saw_objective_flags = false;
    for prior in subrecords[..index].iter().rev() {
        match prior.record_type.as_str() {
            "FNAM" => saw_objective_flags = true,
            "QOBJ" if saw_objective_flags => return true,
            "NNAM" | "CNAM" | "FULL" => return false,
            _ => {}
        }
    }
    false
}

fn write_node(
    node: &PluginNode,
    plugin: &ParsedPlugin,
    dirty_records: &HashSet<usize>,
    output: &mut Vec<u8>,
) -> Result<(), PluginError> {
    match node {
        PluginNode::Raw(bytes) => output.extend_from_slice(bytes),
        PluginNode::Record(index) => {
            let record = &plugin.records[*index];
            if !dirty_records.contains(index) {
                output.extend_from_slice(&record.original);
            } else {
                output.extend(write_record(record, plugin)?);
            }
        }
        PluginNode::Group(group) => {
            if !group_contains_dirty(group, dirty_records) {
                output.extend_from_slice(&group.original);
            } else {
                let mut content = Vec::new();
                for child in &group.children {
                    write_node(child, plugin, dirty_records, &mut content)?;
                }
                let mut header = group.header.clone();
                let len = u32::try_from(GROUP_HEADER_LEN + content.len()).map_err(|_| {
                    PluginError::malformed_plugin(plugin.path(), "group output exceeds u32::MAX")
                })?;
                header[4..8].copy_from_slice(&len.to_le_bytes());
                output.extend(header);
                output.extend(content);
            }
        }
    }
    Ok(())
}

fn write_record(record: &PluginRecord, plugin: &ParsedPlugin) -> Result<Vec<u8>, PluginError> {
    let mut subrecords = Vec::new();
    for subrecord in &record.subrecords {
        if let Some(entry_index) = subrecord.entry_index {
            let entry = &plugin.entries[entry_index];
            if entry.is_dirty() {
                let content = entry_content(entry, plugin)?;
                write_subrecord(
                    &subrecord.record_type,
                    &content,
                    subrecord.overflow,
                    &mut subrecords,
                )?;
                continue;
            }
        }
        subrecords.extend_from_slice(&subrecord.original);
    }

    let content = if record.is_compressed() {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&subrecords).map_err(|err| {
            PluginError::malformed_plugin(
                plugin.path(),
                format!("failed to compress record: {err}"),
            )
        })?;
        let compressed = encoder.finish().map_err(|err| {
            PluginError::malformed_plugin(
                plugin.path(),
                format!("failed to finish compression: {err}"),
            )
        })?;
        let mut content = Vec::with_capacity(4 + compressed.len());
        let uncompressed_len = u32::try_from(subrecords.len()).map_err(|_| {
            PluginError::malformed_plugin(plugin.path(), "uncompressed record exceeds u32::MAX")
        })?;
        content.extend_from_slice(&uncompressed_len.to_le_bytes());
        content.extend(compressed);
        content
    } else {
        subrecords
    };

    let mut header = record.header.clone();
    let content_len = u32::try_from(content.len())
        .map_err(|_| PluginError::malformed_plugin(plugin.path(), "record exceeds u32::MAX"))?;
    header[4..8].copy_from_slice(&content_len.to_le_bytes());
    let mut output = header;
    output.extend(content);
    Ok(output)
}

fn entry_content(
    entry: &PluginLocalizationEntry,
    plugin: &ParsedPlugin,
) -> Result<Vec<u8>, PluginError> {
    if plugin.localized {
        let Some(string_id) = entry.string_id else {
            return Err(PluginError::InvalidLocalizedEntryState {
                path: plugin.path().to_string(),
                record_type: entry.record_type.clone(),
                form_id: entry.form_id,
                subrecord: entry.subrecord.clone(),
                message: "localized plugin entries must be written as string ids".to_string(),
            });
        };
        return Ok(string_id.to_le_bytes().to_vec());
    }
    let text = entry.embedded_text.as_deref().unwrap_or_default();
    let mut bytes = encode_text(
        plugin.options.release,
        plugin.options.embedded_language,
        text,
    )?;
    bytes.push(0);
    Ok(bytes)
}

fn group_contains_dirty(group: &PluginGroup, dirty_records: &HashSet<usize>) -> bool {
    group.children.iter().any(|child| match child {
        PluginNode::Record(index) => dirty_records.contains(index),
        PluginNode::Group(group) => group_contains_dirty(group, dirty_records),
        PluginNode::Raw(_) => false,
    })
}

fn parse_subrecords(path: &str, data: &[u8]) -> Result<Vec<PluginSubrecord>, PluginError> {
    let mut subrecords = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        if data.len() - offset < SUB_HEADER_LEN {
            return Err(PluginError::malformed_plugin(
                path,
                "truncated subrecord header",
            ));
        }
        let record_type = fourcc(&data[offset..offset + 4]);
        let len = read_u16(data, offset + 4) as usize;
        if record_type == "XXXX" {
            if len != 4 || data.len() < offset + SUB_HEADER_LEN + 4 {
                return Err(PluginError::malformed_plugin(
                    path,
                    "malformed XXXX overflow record",
                ));
            }
            let overflow_start = offset;
            let overflow_len = read_u32(data, offset + SUB_HEADER_LEN) as usize;
            offset += SUB_HEADER_LEN + 4;
            if data.len() - offset < SUB_HEADER_LEN {
                return Err(PluginError::malformed_plugin(
                    path,
                    "XXXX overflow record missing following subrecord",
                ));
            }
            let target_type = fourcc(&data[offset..offset + 4]);
            let content_start = offset + SUB_HEADER_LEN;
            let content_end = content_start.checked_add(overflow_len).ok_or_else(|| {
                PluginError::malformed_plugin(path, "XXXX overflow length overflow")
            })?;
            if content_end > data.len() {
                return Err(PluginError::malformed_plugin(
                    path,
                    "XXXX overflow subrecord extends past record data",
                ));
            }
            subrecords.push(PluginSubrecord {
                record_type: target_type,
                original: data[overflow_start..content_end].to_vec(),
                content: data[content_start..content_end].to_vec(),
                overflow: true,
                entry_index: None,
            });
            offset = content_end;
            continue;
        }

        let content_start = offset + SUB_HEADER_LEN;
        let content_end = content_start
            .checked_add(len)
            .ok_or_else(|| PluginError::malformed_plugin(path, "subrecord length overflow"))?;
        if content_end > data.len() {
            return Err(PluginError::malformed_plugin(
                path,
                "subrecord extends past record data",
            ));
        }
        subrecords.push(PluginSubrecord {
            record_type,
            original: data[offset..content_end].to_vec(),
            content: data[content_start..content_end].to_vec(),
            overflow: false,
            entry_index: None,
        });
        offset = content_end;
    }
    Ok(subrecords)
}

fn write_subrecord(
    record_type: &str,
    content: &[u8],
    force_overflow: bool,
    output: &mut Vec<u8>,
) -> Result<(), PluginError> {
    if force_overflow || content.len() > u16::MAX as usize {
        let len = u32::try_from(content.len())
            .map_err(|_| PluginError::malformed_plugin("<memory>", "subrecord exceeds u32::MAX"))?;
        output.extend_from_slice(b"XXXX");
        output.extend_from_slice(&4u16.to_le_bytes());
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(record_type.as_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        output.extend_from_slice(content);
    } else {
        output.extend_from_slice(record_type.as_bytes());
        output.extend_from_slice(&(content.len() as u16).to_le_bytes());
        output.extend_from_slice(content);
    }
    Ok(())
}

fn decompress_record_content(
    content: &[u8],
    content_len: usize,
    path: &str,
) -> Result<Vec<u8>, PluginError> {
    if content_len < 4 {
        return Err(PluginError::malformed_plugin(
            path,
            "compressed record is missing uncompressed length",
        ));
    }
    let expected_len = read_u32(content, 0) as usize;
    if expected_len > MAX_DECOMPRESSED_RECORD_LEN {
        return Err(PluginError::malformed_plugin(
            path,
            "compressed record declares an unreasonable uncompressed length",
        ));
    }
    let decoder = ZlibDecoder::new(&content[4..]);
    let mut limited = decoder.take(expected_len as u64 + 1);
    let mut output = Vec::new();
    output.try_reserve_exact(expected_len).map_err(|err| {
        PluginError::malformed_plugin(
            path,
            format!("failed to reserve decompression buffer: {err}"),
        )
    })?;
    limited.read_to_end(&mut output).map_err(|err| {
        PluginError::malformed_plugin(path, format!("failed to decompress record: {err}"))
    })?;
    if output.len() != expected_len {
        return Err(PluginError::malformed_plugin(
            path,
            "compressed record length does not match declared uncompressed length",
        ));
    }
    Ok(output)
}

fn strip_optional_null(bytes: &[u8]) -> &[u8] {
    bytes.strip_suffix(&[0]).unwrap_or(bytes)
}

fn fourcc(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().expect("u16 slice"))
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("u32 slice"))
}
