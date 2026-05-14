use std::{
    fs,
    io::{BufWriter, Write},
};

use camino::Utf8Path;
use serde::Serialize;
use serde_json::Value;

use crate::{AdaptCatalog, AdaptError, AdaptSummary};

pub fn write_memory_jsonl(
    catalog: &AdaptCatalog,
    path: impl AsRef<Utf8Path>,
) -> Result<AdaptSummary, AdaptError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AdaptError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let file = fs::File::create(path).map_err(|source| AdaptError::WriteFile {
        path: path.to_owned(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    for entry in &catalog.entries {
        let row = MemoryJsonlRow {
            id: &entry.id,
            source: &entry.source,
            target: &entry.target,
            source_locale: &entry.source_locale,
            target_locale: &entry.target_locale,
            context: &entry.context,
            origin: &entry.origin,
            quality: entry.quality.as_str(),
        };
        serde_json::to_writer(&mut writer, &row).map_err(|source| AdaptError::Json {
            path: path.to_owned(),
            source,
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| AdaptError::WriteFile {
                path: path.to_owned(),
                source,
            })?;
    }
    writer.flush().map_err(|source| AdaptError::WriteFile {
        path: path.to_owned(),
        source,
    })?;
    Ok(AdaptSummary {
        total_entries: catalog.summary.total_entries,
        written_entries: catalog.entries.len(),
        skipped_entries: catalog.summary.skipped_entries,
        diagnostics: catalog.diagnostics.len(),
    })
}

#[derive(Serialize)]
struct MemoryJsonlRow<'a> {
    id: &'a str,
    source: &'a str,
    target: &'a str,
    source_locale: &'a str,
    target_locale: &'a str,
    context: &'a std::collections::BTreeMap<String, String>,
    origin: &'a Value,
    quality: &'a str,
}
