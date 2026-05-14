use std::collections::BTreeMap;

use camino::Utf8PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringEntry {
    id: String,
    text: String,
    source: StringEntrySource,
    context: StringEntryContext,
    dirty: bool,
}

impl StringEntry {
    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        source: StringEntrySource,
        context: StringEntryContext,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            source,
            context,
            dirty: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
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

    pub fn source(&self) -> &StringEntrySource {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut StringEntrySource {
        &mut self.source
    }

    pub fn context(&self) -> &StringEntryContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut StringEntryContext {
        &mut self.context
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringEntryContext {
    values: BTreeMap<String, String>,
}

impl StringEntryContext {
    pub fn new(values: BTreeMap<String, String>) -> Self {
        Self { values }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.values.insert(key.into(), value.into())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn values(&self) -> &BTreeMap<String, String> {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringEntrySource {
    Plugin(PluginStringMetadata),
    Pex(PexStringMetadata),
    Scaleform(ScaleformStringMetadata),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginStringMetadata {
    pub path: Utf8PathBuf,
    pub record_type: String,
    pub form_id: u32,
    pub subrecord: String,
    pub strings_kind: String,
    pub field_source: String,
    pub storage: PluginStringStorage,
    pub string_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStringStorage {
    Localized,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexStringMetadata {
    pub path: Utf8PathBuf,
    pub object: String,
    pub state: String,
    pub function: String,
    pub function_kind: PexFunctionKind,
    pub instruction_index: usize,
    pub opcode: String,
    pub operand: PexOperandPath,
    pub string_id: u16,
    pub call_context: Option<PexCallContext>,
    pub concat: Option<PexConcatMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PexFunctionKind {
    Normal,
    Getter,
    Setter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PexOperandPath {
    Fixed(usize),
    Variadic(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexCallContext {
    pub opcode: String,
    pub target: Option<String>,
    pub member: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexConcatMetadata {
    pub group_id: String,
    pub part_index: usize,
    pub ambiguous: bool,
    pub parts: Vec<PexConcatPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PexConcatPart {
    Entry { id: String, text: String },
    Operand { label: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleformStringMetadata {
    pub path: Utf8PathBuf,
    pub key: Option<String>,
}

pub trait StringEntryView {
    fn string_entry(&self) -> &StringEntry;

    fn string_entry_mut(&mut self) -> &mut StringEntry;
}

impl StringEntryView for StringEntry {
    fn string_entry(&self) -> &StringEntry {
        self
    }

    fn string_entry_mut(&mut self) -> &mut StringEntry {
        self
    }
}

pub trait StringEntryBundle {
    type Entry: StringEntryView;

    fn string_entries(&self) -> &[Self::Entry];

    fn string_entries_mut(&mut self) -> &mut [Self::Entry];
}
