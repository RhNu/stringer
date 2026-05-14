use std::collections::BTreeMap;
use std::fs;

use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use stringer_core::{FileAsset, FileBundle};
use tracing::{debug, instrument, trace};
use walkdir::WalkDir;

use crate::archive::{archive_paths, read_archive};
use crate::paths::{
    is_archive_path, is_known_text_asset, logical_path_for_root_relative, normalize_lookup_str,
    utf8_path,
};
use crate::{FileSource, FileSourceKind, FileSourceState, ReaderError};

#[derive(Debug, Clone, Copy, Default)]
pub struct ReadModOptions {
    _reserved: (),
}

#[derive(Debug, Clone)]
pub struct ReadModResult {
    pub files: FileBundle,
    pub sources: Vec<FileSource>,
}

struct PendingAsset {
    asset: FileAsset,
    source_index: usize,
}

#[derive(Default)]
pub(crate) struct AssetAccumulator {
    assets: BTreeMap<String, PendingAsset>,
    sources: Vec<FileSource>,
}

impl AssetAccumulator {
    pub(crate) fn insert(&mut self, asset: FileAsset, kind: FileSourceKind) {
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
