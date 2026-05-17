use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::WorkspaceCoreError;

const BATCHES_DIR: &str = "batches";
pub const BATCH_FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchFile {
    pub schema_version: u32,
    pub batch_format_version: u32,
    pub batch_id: String,
    pub created_at_unix_ms: u128,
    pub revision: u64,
    pub scope: BatchScope,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<BatchEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchEntry {
    pub key: String,
    pub id: String,
}

impl BatchFile {
    pub fn remaining_ids(&self) -> Vec<String> {
        self.entries.iter().cloned().map(|entry| entry.id).collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

pub fn claimed_entry_ids(workspace: &Utf8Path) -> Result<BTreeSet<String>, WorkspaceCoreError> {
    let mut ids = BTreeSet::new();
    for batch in read_batch_files(workspace)? {
        ids.extend(batch.remaining_ids());
    }
    Ok(ids)
}

pub fn claimed_entry_batches(
    workspace: &Utf8Path,
) -> Result<BTreeMap<String, String>, WorkspaceCoreError> {
    let mut claims = BTreeMap::new();
    for batch in read_batch_files(workspace)? {
        for id in batch.remaining_ids() {
            claims.insert(id, batch.batch_id.clone());
        }
    }
    Ok(claims)
}

fn read_batch_files(workspace: &Utf8Path) -> Result<Vec<BatchFile>, WorkspaceCoreError> {
    let dir = workspace.join(BATCHES_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut batches = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|source| WorkspaceCoreError::ReadFile {
        path: dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceCoreError::ReadFile {
            path: dir.clone(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceCoreError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "batch path is not valid UTF-8".to_string(),
            }
        })?;
        if path.extension() == Some("json") {
            let batch: BatchFile = read_json(&path)?;
            validate_batch_format(&path, &batch)?;
            batches.push(batch);
        }
    }
    Ok(batches)
}

pub fn read_batch_file(
    workspace: &Utf8Path,
    batch_id: &str,
) -> Result<BatchFile, WorkspaceCoreError> {
    validate_batch_id(batch_id)?;
    let path = batch_path(workspace, batch_id);
    if !path.exists() {
        return Err(WorkspaceCoreError::BatchNotFound {
            batch_id: batch_id.to_string(),
        });
    }
    let batch: BatchFile = read_json(&path)?;
    validate_batch_format(&path, &batch)?;
    validate_loaded_batch_id(batch_id, &batch.batch_id)?;
    Ok(batch)
}

fn batch_path(workspace: &Utf8Path, batch_id: &str) -> Utf8PathBuf {
    workspace.join(BATCHES_DIR).join(format!("{batch_id}.json"))
}

pub fn validate_batch_id(batch_id: &str) -> Result<(), WorkspaceCoreError> {
    if !batch_id.is_empty() && !batch_id.contains(['/', '\\']) {
        return Ok(());
    }
    Err(WorkspaceCoreError::InvalidTranslationPackagePath {
        path: batch_id.to_string(),
        message: "batch id must be a file name, not a path".to_string(),
    })
}

fn validate_loaded_batch_id(requested: &str, loaded: &str) -> Result<(), WorkspaceCoreError> {
    validate_batch_id(loaded)?;
    if loaded == requested {
        return Ok(());
    }
    Err(WorkspaceCoreError::InvalidTranslationPackagePath {
        path: loaded.to_string(),
        message: format!("batch file id must match requested batch id `{requested}`"),
    })
}

fn validate_batch_format(path: &Utf8Path, batch: &BatchFile) -> Result<(), WorkspaceCoreError> {
    if batch.schema_version != crate::SCHEMA_VERSION {
        return Err(WorkspaceCoreError::UnsupportedTranslationSchema {
            path: path.to_owned(),
            version: batch.schema_version,
        });
    }
    if batch.batch_format_version != BATCH_FORMAT_VERSION {
        return Err(WorkspaceCoreError::UnsupportedBatchFormat {
            path: path.to_owned(),
            version: batch.batch_format_version,
        });
    }
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Utf8Path) -> Result<T, WorkspaceCoreError> {
    let text = fs::read_to_string(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| WorkspaceCoreError::Json {
        path: path.to_owned(),
        source,
    })
}
