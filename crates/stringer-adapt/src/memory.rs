use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
};

use camino::Utf8Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AdaptCatalog, AdaptEntry, AdaptError, AdaptSummary};

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

pub fn merge_memory_jsonl(
    catalog: &AdaptCatalog,
    path: impl AsRef<Utf8Path>,
) -> Result<AdaptSummary, AdaptError> {
    let path = path.as_ref();
    let mut rows = Vec::<Value>::new();
    let mut indexes = BTreeMap::<String, usize>::new();
    if path.exists() {
        let text = fs::read_to_string(path).map_err(|source| AdaptError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let row: Value = serde_json::from_str(line).map_err(|source| AdaptError::Json {
                path: path.to_owned(),
                source,
            })?;
            let id = memory_id(row.clone(), path)?;
            if let Some(index) = indexes.get(&id).copied() {
                rows[index] = row;
            } else {
                indexes.insert(id, rows.len());
                rows.push(row);
            }
        }
    }
    for entry in &catalog.entries {
        let row = memory_row_value(entry, path)?;
        if let Some(index) = indexes.get(entry.id.as_str()).copied() {
            rows[index] = row;
        } else {
            indexes.insert(entry.id.clone(), rows.len());
            rows.push(row);
        }
    }
    write_rows(&rows, path)?;
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

#[derive(Deserialize)]
struct MemoryJsonlId {
    id: String,
}

fn memory_row_value(entry: &AdaptEntry, path: &Utf8Path) -> Result<Value, AdaptError> {
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
    serde_json::to_value(row).map_err(|source| AdaptError::Json {
        path: path.to_owned(),
        source,
    })
}

fn memory_id(row: Value, path: &Utf8Path) -> Result<String, AdaptError> {
    serde_json::from_value::<MemoryJsonlId>(row)
        .map(|row| row.id)
        .map_err(|source| AdaptError::Json {
            path: path.to_owned(),
            source,
        })
}

fn write_rows(rows: &[Value], path: &Utf8Path) -> Result<(), AdaptError> {
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
    for row in rows {
        serde_json::to_writer(&mut writer, row).map_err(|source| AdaptError::Json {
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
    })
}
