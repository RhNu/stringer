use camino::Utf8PathBuf;
use stringer_app::WorkspaceBatchSubmitActionInput;

use crate::app::{CliError, read_input};

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WorkspaceBatchSubmitPatchInput {
    pub(crate) batch_id: String,
    pub(crate) revision: u64,
    pub(crate) entries: Vec<WorkspaceBatchSubmitPatchEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct WorkspaceBatchSubmitPatchEntry {
    pub(crate) key: String,
    pub(crate) action: WorkspaceBatchSubmitActionInput,
    #[serde(default)]
    pub(crate) translation: Option<String>,
    #[serde(default)]
    pub(crate) skip_reason: Option<String>,
}

pub(crate) fn read_batch_submit_patch(
    input: &Utf8PathBuf,
) -> Result<WorkspaceBatchSubmitPatchInput, CliError> {
    let text = read_input(input)?;
    if input.as_str() != "-" && input.extension() == Some("csv") {
        return parse_batch_submit_csv(input, &text);
    }
    serde_json::from_str(&text).map_err(|source| CliError::Json {
        path: input.clone(),
        source,
    })
}

fn parse_batch_submit_csv(
    path: &Utf8PathBuf,
    text: &str,
) -> Result<WorkspaceBatchSubmitPatchInput, CliError> {
    let mut batch_id = None;
    let mut revision = None;
    let (metadata_line, csv_text) =
        split_first_line(text).ok_or_else(|| CliError::InvalidArguments {
            message: format!("CSV patch `{path}` is empty"),
        })?;
    if let Some(metadata) = metadata_line.strip_prefix("# stringer ") {
        for item in metadata.split_whitespace() {
            if let Some(value) = item.strip_prefix("batch_id=") {
                batch_id = Some(value.to_string());
            } else if let Some(value) = item.strip_prefix("revision=") {
                revision = value.parse::<u64>().ok();
            }
        }
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(csv_text.as_bytes());
    let columns = reader
        .headers()
        .map_err(|source| CliError::InvalidArguments {
            message: format!("failed to parse CSV patch `{path}` header: {source}"),
        })?
        .clone();
    let key_index = csv_column(&columns, "key")?;
    let action_index = csv_column(&columns, "action")?;
    let translation_index = csv_column(&columns, "translation")?;
    let skip_reason_index = csv_column(&columns, "skip_reason")?;
    let mut entries = Vec::new();
    for row in reader.records() {
        let row = row.map_err(|source| CliError::InvalidArguments {
            message: format!("failed to parse CSV patch `{path}` row: {source}"),
        })?;
        let action = match csv_get(&row, action_index) {
            "translate" => WorkspaceBatchSubmitActionInput::Translate,
            "skip" => WorkspaceBatchSubmitActionInput::Skip,
            "pending" | "" => WorkspaceBatchSubmitActionInput::Pending,
            other => {
                return Err(CliError::InvalidArguments {
                    message: format!("unsupported batch submit action `{other}` in `{path}`"),
                });
            }
        };
        entries.push(WorkspaceBatchSubmitPatchEntry {
            key: csv_get(&row, key_index).to_string(),
            action,
            translation: non_empty(csv_get(&row, translation_index)),
            skip_reason: non_empty(csv_get(&row, skip_reason_index)),
        });
    }
    Ok(WorkspaceBatchSubmitPatchInput {
        batch_id: batch_id.ok_or_else(|| CliError::InvalidArguments {
            message: format!("CSV patch `{path}` is missing batch_id metadata"),
        })?,
        revision: revision.ok_or_else(|| CliError::InvalidArguments {
            message: format!("CSV patch `{path}` is missing revision metadata"),
        })?,
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

fn csv_column(columns: &csv::StringRecord, name: &str) -> Result<usize, CliError> {
    columns
        .iter()
        .position(|column| column == name)
        .ok_or_else(|| CliError::InvalidArguments {
            message: format!("CSV patch is missing `{name}` column"),
        })
}

fn csv_get(row: &csv::StringRecord, index: usize) -> &str {
    row.get(index).unwrap_or("")
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_patch_parser_accepts_quoted_multiline_fields() {
        let patch = concat!(
            "# stringer batch_id=b-test revision=7\n",
            "key,source,current_translation,context_label,diagnostic_codes,action,translation,skip_reason\n",
            "e001,\"Iron\nSword\",,,memory.conflict,translate,\"熟\n铁剑\",\n",
        );

        let parsed = parse_batch_submit_csv(&Utf8PathBuf::from("patch.csv"), patch).unwrap();

        assert_eq!(parsed.batch_id, "b-test");
        assert_eq!(parsed.revision, 7);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].key, "e001");
        assert_eq!(parsed.entries[0].translation.as_deref(), Some("熟\n铁剑"));
    }
}
