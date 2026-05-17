use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde::Deserialize;

use crate::WorkspaceOpsError;
use crate::batch_packet::{BatchSubmitAction, BatchSubmitEntry, BatchSubmitOptions, SKIP_REASONS};

const DEFAULT_SUBMISSION_REVISION: u64 = 1;

impl BatchSubmitOptions {
    pub fn from_submission_file(
        workspace: Utf8PathBuf,
        path: Utf8PathBuf,
    ) -> Result<Self, WorkspaceOpsError> {
        let text = fs::read_to_string(&path).map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::ReadFile {
                path: path.clone(),
                source,
            }
        })?;
        Self::from_submission_text(workspace, &path, &text)
    }

    pub fn from_submission_text(
        workspace: Utf8PathBuf,
        path: &Utf8Path,
        text: &str,
    ) -> Result<Self, WorkspaceOpsError> {
        if path.extension() == Some("csv") {
            return parse_batch_submit_csv(workspace, path, text);
        }
        let submission: BatchSubmitInput = serde_json::from_str(text).map_err(|source| {
            stringer_workspace_core::WorkspaceCoreError::Json {
                path: path.to_owned(),
                source,
            }
        })?;
        validate_submit_skip_reasons(path, &submission.entries)?;
        Ok(Self {
            workspace,
            batch_id: submission.batch_id,
            revision: submission.revision,
            entries: submission.entries,
        })
    }

    pub fn from_json_file(workspace: Utf8PathBuf, path: String) -> Result<Self, WorkspaceOpsError> {
        Self::from_submission_file(workspace, Utf8PathBuf::from(path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BatchSubmitInput {
    batch_id: String,
    #[serde(default = "default_submission_revision")]
    revision: u64,
    entries: Vec<BatchSubmitEntry>,
}

fn default_submission_revision() -> u64 {
    DEFAULT_SUBMISSION_REVISION
}

fn parse_batch_submit_csv(
    workspace: Utf8PathBuf,
    path: &Utf8Path,
    text: &str,
) -> Result<BatchSubmitOptions, WorkspaceOpsError> {
    let mut batch_id = None;
    let mut revision = None;
    let (metadata_line, csv_text) = split_first_line(text)
        .ok_or_else(|| invalid_submission(path, format!("CSV submission `{path}` is empty")))?;
    let Some(metadata) = metadata_line.strip_prefix("# stringer ") else {
        return Err(invalid_submission(
            path,
            format!("CSV submission `{path}` is missing batch_id metadata"),
        ));
    };
    for item in metadata.split_whitespace() {
        if let Some(value) = item.strip_prefix("batch_id=") {
            batch_id = Some(value.to_string());
        } else if let Some(value) = item.strip_prefix("revision=") {
            revision = value.parse::<u64>().ok();
        }
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let columns = reader.headers().map_err(|source| {
        invalid_submission(
            path,
            format!("failed to parse CSV submission `{path}` header: {source}"),
        )
    })?;
    let key_index = csv_column(path, columns, "key")?;
    let action_index = csv_column(path, columns, "action")?;
    let translation_index = csv_column(path, columns, "translation")?;
    let skip_reason_index = csv_column(path, columns, "skip_reason")?;
    let mut entries = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|source| {
            invalid_submission(
                path,
                format!("failed to parse CSV submission `{path}` row: {source}"),
            )
        })?;
        entries.push(BatchSubmitEntry {
            key: csv_get(&row, key_index).to_string(),
            action: parse_csv_action(path, csv_get(&row, action_index))?,
            translation: non_empty(csv_get(&row, translation_index)),
            skip_reason: parse_csv_skip_reason(path, csv_get(&row, skip_reason_index))?,
        });
    }
    Ok(BatchSubmitOptions {
        workspace,
        batch_id: batch_id.ok_or_else(|| {
            invalid_submission(
                path,
                format!("CSV submission `{path}` is missing batch_id metadata"),
            )
        })?,
        revision: revision.unwrap_or(DEFAULT_SUBMISSION_REVISION),
        entries,
    })
}

fn split_first_line(text: &str) -> Option<(&str, &str)> {
    if text.is_empty() {
        return None;
    }
    if let Some(index) = text.find('\n') {
        let first = text[..index].strip_suffix('\r').unwrap_or(&text[..index]);
        return Some((first, &text[index + 1..]));
    }
    Some((text.strip_suffix('\r').unwrap_or(text), ""))
}

fn csv_column(
    path: &Utf8Path,
    columns: &csv::StringRecord,
    name: &str,
) -> Result<usize, WorkspaceOpsError> {
    columns
        .iter()
        .position(|column| column == name)
        .ok_or_else(|| {
            invalid_submission(path, format!("CSV submission is missing `{name}` column"))
        })
}

fn csv_get(row: &csv::StringRecord, index: usize) -> &str {
    row.get(index).unwrap_or("")
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn parse_csv_action(path: &Utf8Path, value: &str) -> Result<BatchSubmitAction, WorkspaceOpsError> {
    match value {
        "translate" => Ok(BatchSubmitAction::Translate),
        "skip" => Ok(BatchSubmitAction::Skip),
        "pending" | "" => Ok(BatchSubmitAction::Pending),
        other => Err(invalid_submission(
            path,
            format!("unsupported batch submit action `{other}` in `{path}`"),
        )),
    }
}

fn parse_csv_skip_reason(
    path: &Utf8Path,
    value: &str,
) -> Result<Option<String>, WorkspaceOpsError> {
    match value {
        "" => Ok(None),
        value if SKIP_REASONS.contains(&value) => Ok(Some(value.to_string())),
        other => Err(invalid_submission(
            path,
            format!("unsupported skip_reason `{other}` in `{path}`"),
        )),
    }
}

fn validate_submit_skip_reasons(
    path: &Utf8Path,
    entries: &[BatchSubmitEntry],
) -> Result<(), WorkspaceOpsError> {
    for entry in entries {
        if let Some(reason) = &entry.skip_reason
            && !SKIP_REASONS.contains(&reason.as_str())
        {
            return Err(invalid_submission(
                path,
                format!("unsupported skip_reason `{reason}` in `{path}`"),
            ));
        }
    }
    Ok(())
}

fn invalid_submission(path: &Utf8Path, message: String) -> WorkspaceOpsError {
    stringer_workspace_core::WorkspaceCoreError::InvalidTranslationPackagePath {
        path: path.to_string(),
        message,
    }
    .into()
}
