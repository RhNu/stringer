//! Shared foundations for Stringer crates.

use std::collections::HashMap;

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
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileRole {
    Plugin,
    Strings,
    Unknown,
}

impl FileRole {
    pub fn from_format(format: FileFormat) -> Self {
        match format {
            FileFormat::Esp | FileFormat::Esm | FileFormat::Esl => Self::Plugin,
            FileFormat::Strings | FileFormat::DlStrings | FileFormat::IlStrings => Self::Strings,
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
