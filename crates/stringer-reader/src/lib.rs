#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io;

use ba2::ByteSlice as _;
use ba2::prelude::*;
use bytes::Bytes;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use stringer_core::{FileAsset, FileBundle, FileRole, StringerCoreError};
use thiserror::Error;
use tracing::{debug, instrument, trace};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Default)]
pub struct ReadModOptions {
    _reserved: (),
}

#[derive(Debug, Clone)]
pub struct ReadModResult {
    pub files: FileBundle,
    pub sources: Vec<FileSource>,
}

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

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to walk `{path}`: {source}")]
    Walk {
        path: String,
        #[source]
        source: walkdir::Error,
    },

    #[error("unsupported archive format `{path}`")]
    UnsupportedArchive { path: Utf8PathBuf },

    #[error("failed to read archive `{path}`: {message}")]
    Archive { path: Utf8PathBuf, message: String },

    #[error(transparent)]
    Bundle(#[from] StringerCoreError),
}

struct PendingAsset {
    asset: FileAsset,
    source_index: usize,
}

#[derive(Default)]
struct AssetAccumulator {
    assets: BTreeMap<String, PendingAsset>,
    sources: Vec<FileSource>,
}

impl AssetAccumulator {
    fn insert(&mut self, asset: FileAsset, kind: FileSourceKind) {
        let key = normalize_lookup_str(asset.path().as_str());
        if let Some(previous) = self.assets.get(&key) {
            self.sources[previous.source_index].state = FileSourceState::Shadowed;
        }

        let source_index = self.sources.len();
        self.sources.push(FileSource {
            logical_path: asset.path().to_owned(),
            kind,
            state: FileSourceState::Included,
        });
        self.assets.insert(
            key,
            PendingAsset {
                asset,
                source_index,
            },
        );
    }

    fn finish(self) -> Result<ReadModResult, ReaderError> {
        let files = self
            .assets
            .into_values()
            .map(|pending| pending.asset)
            .collect();
        Ok(ReadModResult {
            files: FileBundle::try_new(files)?,
            sources: self.sources,
        })
    }
}

#[instrument(skip(_options), fields(root = %root.as_ref()))]
pub fn read_mod_root(
    root: impl AsRef<Utf8Path>,
    _options: ReadModOptions,
) -> Result<ReadModResult, ReaderError> {
    let root = root.as_ref();
    let mut accumulator = AssetAccumulator::default();

    for archive_path in archive_paths(root)? {
        read_archive(&archive_path, &mut accumulator)?;
    }
    for loose_path in loose_file_paths(root)? {
        read_loose_file(root, &loose_path, &mut accumulator)?;
    }

    debug!(sources = accumulator.sources.len(), "read mod root");
    accumulator.finish()
}

fn archive_paths(root: &Utf8Path) -> Result<Vec<Utf8PathBuf>, ReaderError> {
    let mut paths = Vec::new();
    collect_archive_paths(root, &mut paths)?;
    let data = root.join("Data");
    if data.is_dir() {
        collect_archive_paths(&data, &mut paths)?;
    }
    paths.sort_by_key(|path| normalize_lookup_str(path.as_str()));
    paths.dedup_by_key(|path| normalize_lookup_str(path.as_str()));
    Ok(paths)
}

fn collect_archive_paths(
    directory: &Utf8Path,
    paths: &mut Vec<Utf8PathBuf>,
) -> Result<(), ReaderError> {
    let entries = fs::read_dir(directory).map_err(|source| ReaderError::Io {
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| ReaderError::Io {
            path: directory.to_owned(),
            source,
        })?;
        let path = utf8_path(entry.path())?;
        if path.is_file() && is_archive_path(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

fn loose_file_paths(root: &Utf8Path) -> Result<Vec<Utf8PathBuf>, ReaderError> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|source| ReaderError::Walk {
            path: root.to_string(),
            source,
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = utf8_path(entry.path().to_path_buf())?;
        if is_archive_path(&path) {
            continue;
        }
        let logical_path = logical_path_for_root_relative(root, &path)?;
        if is_known_text_asset(&logical_path) {
            paths.push(path);
        }
    }
    paths.sort_by_key(|path| normalize_lookup_str(path.as_str()));
    Ok(paths)
}

fn read_loose_file(
    root: &Utf8Path,
    path: &Utf8Path,
    accumulator: &mut AssetAccumulator,
) -> Result<(), ReaderError> {
    let logical_path = logical_path_for_root_relative(root, path)?;
    let bytes = fs::read(path).map_err(|source| ReaderError::Io {
        path: path.to_owned(),
        source,
    })?;
    let asset = FileAsset::new(logical_path, Bytes::from(bytes));
    if is_known_text_asset(asset.path()) {
        trace!(path = %path, logical_path = %asset.path(), "read loose asset");
        accumulator.insert(
            asset,
            FileSourceKind::Loose {
                path: path.to_owned(),
            },
        );
    }
    Ok(())
}

fn read_archive(path: &Utf8Path, accumulator: &mut AssetAccumulator) -> Result<(), ReaderError> {
    let mut file = fs::File::open(path).map_err(|source| ReaderError::Io {
        path: path.to_owned(),
        source,
    })?;
    let format = ba2::guess_format(&mut file)
        .ok_or_else(|| ReaderError::UnsupportedArchive { path: path.into() })?;

    match format {
        ba2::FileFormat::TES3 => read_tes3_archive(path, accumulator),
        ba2::FileFormat::TES4 => read_tes4_archive(path, accumulator),
        ba2::FileFormat::FO4 => read_fo4_archive(path, accumulator),
    }
}

fn read_tes3_archive(
    path: &Utf8Path,
    accumulator: &mut AssetAccumulator,
) -> Result<(), ReaderError> {
    let archive =
        ba2::tes3::Archive::read(path.as_std_path()).map_err(|error| archive_error(path, error))?;
    for (key, file) in archive.iter() {
        let entry_path = archive_entry_path(key.name());
        let Some(logical_path) = logical_path_for_archive_entry(&entry_path) else {
            trace!(archive_path = %path, entry_path = %entry_path, "skipping invalid archive path");
            continue;
        };
        if !is_known_text_asset(&logical_path) {
            continue;
        }
        insert_archive_asset(
            accumulator,
            path,
            entry_path,
            logical_path,
            file.as_bytes().to_vec(),
        );
    }
    Ok(())
}

fn read_tes4_archive(
    path: &Utf8Path,
    accumulator: &mut AssetAccumulator,
) -> Result<(), ReaderError> {
    let (archive, options) =
        ba2::tes4::Archive::read(path.as_std_path()).map_err(|error| archive_error(path, error))?;
    let file_options = ba2::tes4::FileCompressionOptions::from(options);
    for (directory_key, directory) in archive.iter() {
        for (file_key, file) in directory.iter() {
            let entry_path = archive_entry_path_joined(directory_key.name(), file_key.name());
            let Some(logical_path) = logical_path_for_archive_entry(&entry_path) else {
                trace!(archive_path = %path, entry_path = %entry_path, "skipping invalid archive path");
                continue;
            };
            if !is_known_text_asset(&logical_path) {
                continue;
            }
            let mut bytes = Vec::new();
            file.write(&mut bytes, &file_options)
                .map_err(|error| archive_error(path, error))?;
            insert_archive_asset(accumulator, path, entry_path, logical_path, bytes);
        }
    }
    Ok(())
}

fn read_fo4_archive(
    path: &Utf8Path,
    accumulator: &mut AssetAccumulator,
) -> Result<(), ReaderError> {
    let (archive, options) =
        ba2::fo4::Archive::read(path.as_std_path()).map_err(|error| archive_error(path, error))?;
    let file_options = ba2::fo4::FileWriteOptions::from(options);
    for (key, file) in archive.iter() {
        if key.name().is_empty() {
            continue;
        }
        let entry_path = archive_entry_path(key.name());
        let Some(logical_path) = logical_path_for_archive_entry(&entry_path) else {
            trace!(archive_path = %path, entry_path = %entry_path, "skipping invalid archive path");
            continue;
        };
        if !is_known_text_asset(&logical_path) {
            continue;
        }
        let mut bytes = Vec::new();
        file.write(&mut bytes, &file_options)
            .map_err(|error| archive_error(path, error))?;
        insert_archive_asset(accumulator, path, entry_path, logical_path, bytes);
    }
    Ok(())
}

fn insert_archive_asset(
    accumulator: &mut AssetAccumulator,
    archive_path: &Utf8Path,
    entry_path: Utf8PathBuf,
    logical_path: Utf8PathBuf,
    bytes: Vec<u8>,
) {
    let asset = FileAsset::new(logical_path, Bytes::from(bytes));
    trace!(
        archive_path = %archive_path,
        entry_path = %entry_path,
        logical_path = %asset.path(),
        "read archive asset"
    );
    accumulator.insert(
        asset,
        FileSourceKind::Archive {
            archive_path: archive_path.to_owned(),
            entry_path,
        },
    );
}

fn logical_path_for_root_relative(
    root: &Utf8Path,
    path: &Utf8Path,
) -> Result<Utf8PathBuf, ReaderError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|source| ReaderError::Archive {
            path: path.to_owned(),
            message: source.to_string(),
        })?;
    logical_path_from_virtual_path(relative).ok_or_else(|| ReaderError::Archive {
        path: path.to_owned(),
        message: "path contains invalid virtual path components".to_string(),
    })
}

fn logical_path_for_archive_entry(entry_path: &Utf8Path) -> Option<Utf8PathBuf> {
    logical_path_from_virtual_path(entry_path)
}

fn logical_path_from_virtual_path(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let mut parts = Vec::new();
    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Utf8Component::Normal(part) => {
                if !saw_normal && part.eq_ignore_ascii_case("Data") {
                    saw_normal = true;
                    continue;
                }
                saw_normal = true;
                parts.push(part);
            }
            Utf8Component::Prefix(_)
            | Utf8Component::RootDir
            | Utf8Component::CurDir
            | Utf8Component::ParentDir => return None,
        }
    }

    if parts.is_empty() {
        return None;
    }

    let mut logical_path = Utf8PathBuf::from("Data");
    for part in parts {
        logical_path.push(part);
    }
    Some(logical_path)
}

fn archive_entry_path(path: &ba2::BStr) -> Utf8PathBuf {
    Utf8PathBuf::from(path.to_str_lossy().replace('\\', "/"))
}

fn archive_entry_path_joined(directory: &ba2::BStr, file: &ba2::BStr) -> Utf8PathBuf {
    let directory = directory.to_str_lossy();
    let file = file.to_str_lossy();
    if directory.is_empty() || directory == "." || directory == "/" || directory == "\\" {
        return Utf8PathBuf::from(file.replace('\\', "/"));
    }
    Utf8PathBuf::from(format!(
        "{}/{}",
        directory.replace('\\', "/"),
        file.replace('\\', "/")
    ))
}

fn normalize_lookup_str(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn is_archive_path(path: &Utf8Path) -> bool {
    matches!(
        path.extension()
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("bsa" | "ba2")
    )
}

fn is_known_text_asset(path: &Utf8Path) -> bool {
    matches!(
        FileRole::from_format(stringer_core::FileFormat::from_path(path)),
        FileRole::Plugin | FileRole::Strings | FileRole::Pex | FileRole::Scaleform
    )
}

fn utf8_path(path: std::path::PathBuf) -> Result<Utf8PathBuf, ReaderError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| ReaderError::Archive {
        path: Utf8PathBuf::from(path.to_string_lossy().as_ref()),
        message: "path is not valid UTF-8".to_string(),
    })
}

fn archive_error(path: &Utf8Path, error: impl std::error::Error) -> ReaderError {
    ReaderError::Archive {
        path: path.to_owned(),
        message: error.to_string(),
    }
}
