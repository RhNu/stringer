use camino::Utf8PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSource {
    pub logical_path: Utf8PathBuf,
    pub kind: FileSourceKind,
    pub state: FileSourceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSourceKind {
    Loose {
        path: Utf8PathBuf,
    },
    Archive {
        archive_path: Utf8PathBuf,
        entry_path: Utf8PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSourceState {
    Included,
    Shadowed,
}
