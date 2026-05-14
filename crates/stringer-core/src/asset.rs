use std::collections::HashMap;

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use tracing::{debug, instrument, trace};

use crate::StringerCoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileFormat {
    Esp,
    Esm,
    Esl,
    Strings,
    DlStrings,
    IlStrings,
    Pex,
    ScaleformTranslation,
    Unknown,
}

impl FileFormat {
    #[instrument(level = "trace", skip(path), fields(path = %path.as_ref()))]
    pub fn from_path(path: impl AsRef<Utf8Path>) -> Self {
        let path = path.as_ref();
        let format = match path
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
            Some("txt") if is_scaleform_translation_path(path) => Self::ScaleformTranslation,
            _ => Self::Unknown,
        };
        trace!(?format, "classified file format");
        format
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileRole {
    Plugin,
    Strings,
    Pex,
    Scaleform,
    Unknown,
}

impl FileRole {
    #[instrument(level = "trace", fields(?format))]
    pub fn from_format(format: FileFormat) -> Self {
        let role = match format {
            FileFormat::Esp | FileFormat::Esm | FileFormat::Esl => Self::Plugin,
            FileFormat::Strings | FileFormat::DlStrings | FileFormat::IlStrings => Self::Strings,
            FileFormat::Pex => Self::Pex,
            FileFormat::ScaleformTranslation => Self::Scaleform,
            FileFormat::Unknown => Self::Unknown,
        };
        trace!(?role, "classified file role");
        role
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
    #[instrument(level = "trace", skip(path, bytes))]
    pub fn new(path: impl Into<Utf8PathBuf>, bytes: Bytes) -> Self {
        let path = path.into();
        let format = FileFormat::from_path(&path);
        let role = FileRole::from_format(format);
        trace!(?format, ?role, len = bytes.len(), "created file asset");
        Self {
            path,
            bytes,
            role,
            format,
        }
    }

    #[instrument(level = "trace", skip(path, bytes), fields(?role))]
    pub fn with_role(path: impl Into<Utf8PathBuf>, bytes: Bytes, role: FileRole) -> Self {
        let path = path.into();
        let format = FileFormat::from_path(&path);
        trace!(
            ?format,
            len = bytes.len(),
            "created file asset with explicit role"
        );
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
    #[instrument(skip(files), fields(files = files.len()))]
    pub fn new(files: Vec<FileAsset>) -> Self {
        Self::try_new(files).expect("file bundle contains duplicate logical paths")
    }

    #[instrument(skip(files), fields(files = files.len()))]
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
        debug!(files = files.len(), "created file bundle");
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

    pub fn scaleform(&self) -> impl Iterator<Item = &FileAsset> {
        self.files
            .iter()
            .filter(|asset| asset.role() == FileRole::Scaleform)
    }
}

fn normalize_lookup_key(path: &Utf8Path) -> String {
    normalize_lookup_str(path.as_str())
}

fn normalize_lookup_str(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn is_scaleform_translation_path(path: &Utf8Path) -> bool {
    let normalized = normalize_lookup_key(path);
    let Some(rest) = normalized.strip_prefix("data/interface/translations/") else {
        return false;
    };
    !rest.is_empty() && !rest.contains('/')
}
