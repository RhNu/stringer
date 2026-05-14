use std::fs;

use ba2::ByteSlice as _;
use ba2::prelude::*;
use bytes::Bytes;
use camino::{Utf8Path, Utf8PathBuf};
use stringer_core::FileAsset;
use tracing::trace;

use crate::paths::{
    is_archive_path, is_known_text_asset, logical_path_for_archive_entry, normalize_lookup_str,
    utf8_path,
};
use crate::reader::AssetAccumulator;
use crate::{FileSourceKind, ReaderError};

pub(crate) fn archive_paths(root: &Utf8Path) -> Result<Vec<Utf8PathBuf>, ReaderError> {
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

pub(crate) fn read_archive(
    path: &Utf8Path,
    accumulator: &mut AssetAccumulator,
) -> Result<(), ReaderError> {
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

fn archive_error(path: &Utf8Path, error: impl std::error::Error) -> ReaderError {
    ReaderError::Archive {
        path: path.to_owned(),
        message: error.to_string(),
    }
}
