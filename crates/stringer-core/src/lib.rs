//! Shared foundations for Stringer crates.

use std::collections::{BTreeMap, HashMap};

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileFormat {
    Esp,
    Esm,
    Esl,
    Strings,
    DlStrings,
    IlStrings,
    Pex,
    Unknown,
}

impl FileFormat {
    pub fn from_path(path: impl AsRef<Utf8Path>) -> Self {
        match path
            .as_ref()
            .extension()
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("esp") => Self::Esp,
            Some("esm") => Self::Esm,
            Some("esl") => Self::Esl,
            Some("strings") => Self::Strings,
            Some("dlstrings") => Self::DlStrings,
            Some("ilstrings") => Self::IlStrings,
            Some("pex") => Self::Pex,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileRole {
    Plugin,
    Strings,
    Pex,
    Unknown,
}

impl FileRole {
    pub fn from_format(format: FileFormat) -> Self {
        match format {
            FileFormat::Esp | FileFormat::Esm | FileFormat::Esl => Self::Plugin,
            FileFormat::Strings | FileFormat::DlStrings | FileFormat::IlStrings => Self::Strings,
            FileFormat::Pex => Self::Pex,
            FileFormat::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileAsset {
    path: Utf8PathBuf,
    bytes: Bytes,
    role: FileRole,
    format: FileFormat,
}

impl FileAsset {
    pub fn new(path: impl Into<Utf8PathBuf>, bytes: Bytes) -> Self {
        let path = path.into();
        let format = FileFormat::from_path(&path);
        let role = FileRole::from_format(format);
        Self {
            path,
            bytes,
            role,
            format,
        }
    }

    pub fn with_role(path: impl Into<Utf8PathBuf>, bytes: Bytes, role: FileRole) -> Self {
        let path = path.into();
        let format = FileFormat::from_path(&path);
        Self {
            path,
            bytes,
            role,
            format,
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn bytes(&self) -> &Bytes {
        &self.bytes
    }

    pub fn into_bytes(self) -> Bytes {
        self.bytes
    }

    pub fn role(&self) -> FileRole {
        self.role
    }

    pub fn format(&self) -> FileFormat {
        self.format
    }
}

#[derive(Debug, Clone)]
pub struct FileBundle {
    files: Vec<FileAsset>,
    lookup: HashMap<String, usize>,
}

impl FileBundle {
    pub fn new(files: Vec<FileAsset>) -> Self {
        Self::try_new(files).expect("file bundle contains duplicate logical paths")
    }

    pub fn try_new(files: Vec<FileAsset>) -> Result<Self, StringerCoreError> {
        let mut lookup = HashMap::with_capacity(files.len());
        for (index, file) in files.iter().enumerate() {
            let key = normalize_lookup_key(file.path());
            if lookup.insert(key.clone(), index).is_some() {
                return Err(StringerCoreError::DuplicatePath {
                    path: file.path().to_string(),
                });
            }
        }
        Ok(Self { files, lookup })
    }

    pub fn files(&self) -> impl Iterator<Item = &FileAsset> {
        self.files.iter()
    }

    pub fn into_files(self) -> Vec<FileAsset> {
        self.files
    }

    pub fn get(&self, path: impl AsRef<str>) -> Option<&FileAsset> {
        self.lookup
            .get(&normalize_lookup_str(path.as_ref()))
            .map(|index| &self.files[*index])
    }

    pub fn plugins(&self) -> impl Iterator<Item = &FileAsset> {
        self.files
            .iter()
            .filter(|asset| asset.role() == FileRole::Plugin)
    }

    pub fn strings(&self) -> impl Iterator<Item = &FileAsset> {
        self.files
            .iter()
            .filter(|asset| asset.role() == FileRole::Strings)
    }

    pub fn pex(&self) -> impl Iterator<Item = &FileAsset> {
        self.files
            .iter()
            .filter(|asset| asset.role() == FileRole::Pex)
    }
}

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
    Mcm(McmStringMetadata),
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
pub struct McmStringMetadata {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    path: Utf8PathBuf,
    offset: usize,
    len: usize,
}

impl SourceSpan {
    pub fn new(path: impl Into<Utf8PathBuf>, offset: usize, len: usize) -> Self {
        Self {
            path: path.into(),
            offset,
            len,
        }
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    severity: DiagnosticSeverity,
    message: String,
    span: Option<SourceSpan>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<SourceSpan>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            span,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }

    pub fn span(&self) -> Option<&SourceSpan> {
        self.span.as_ref()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StringerCoreError {
    #[error("duplicate logical file path in bundle: {path}")]
    DuplicatePath { path: String },
}

fn normalize_lookup_key(path: &Utf8Path) -> String {
    normalize_lookup_str(path.as_str())
}

fn normalize_lookup_str(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}
