use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use thiserror::Error;

use crate::hash::adapt_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptFormat {
    EetBinary,
    EetXml,
    EetJson,
    XtSst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptImportOptions {
    pub source_locale: String,
    pub target_locale: String,
    pub game: Option<String>,
    pub format: AdaptFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptCatalog {
    pub entries: Vec<AdaptEntry>,
    pub diagnostics: Vec<AdaptDiagnostic>,
    pub summary: AdaptSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptEntry {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_locale: String,
    pub target_locale: String,
    pub context: BTreeMap<String, String>,
    pub origin: Value,
    pub quality: AdaptQuality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptDiagnostic {
    pub code: String,
    pub message: String,
    pub entry: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdaptSummary {
    pub total_entries: usize,
    pub written_entries: usize,
    pub skipped_entries: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptQuality {
    Confirmed,
    Imported,
    Machine,
    Rejected,
}

impl AdaptQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Imported => "imported",
            Self::Machine => "machine",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Error)]
pub enum AdaptError {
    #[error("failed to read `{path}`: {source}")]
    ReadFile {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write `{path}`: {source}")]
    WriteFile {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse JSON `{path}`: {source}")]
    Json {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("malformed {format} data in `{path}`: {message}")]
    Malformed {
        path: Utf8PathBuf,
        format: &'static str,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedEntry {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) context: BTreeMap<String, String>,
    pub(crate) origin: Value,
    pub(crate) quality: AdaptQuality,
}

pub(crate) fn catalog_from_entries(
    format: &'static str,
    parsed: Vec<ParsedEntry>,
    options: &AdaptImportOptions,
    game: Option<String>,
) -> AdaptCatalog {
    let mut entries = Vec::new();
    let mut diagnostics = Vec::new();
    let total_entries = parsed.len();
    let mut ids = BTreeMap::<String, usize>::new();
    for (index, mut entry) in parsed.into_iter().enumerate() {
        if let Some(game) = game
            .as_ref()
            .or(options.game.as_ref())
            .and_then(|game| canonical_game_context(game))
        {
            entry
                .context
                .entry("game".to_string())
                .or_insert(game.clone());
        }
        if entry.source.trim().is_empty() || entry.target.trim().is_empty() {
            diagnostics.push(AdaptDiagnostic {
                code: "adapt.empty_text".to_string(),
                message: "source or target text is empty".to_string(),
                entry: Some(index),
            });
            continue;
        }
        let base_id = adapt_id(format, &entry.source, &entry.target, &entry.context);
        let count = ids.entry(base_id.clone()).or_default();
        let id = if *count == 0 {
            base_id
        } else {
            format!("{base_id}:{}", *count)
        };
        *count += 1;
        entries.push(AdaptEntry {
            id,
            source: entry.source,
            target: entry.target,
            source_locale: options.source_locale.clone(),
            target_locale: options.target_locale.clone(),
            context: entry.context,
            origin: entry.origin,
            quality: entry.quality,
        });
    }
    AdaptCatalog {
        summary: AdaptSummary {
            total_entries,
            written_entries: entries.len(),
            skipped_entries: total_entries - entries.len(),
            diagnostics: diagnostics.len(),
        },
        entries,
        diagnostics,
    }
}

pub(crate) fn insert_non_empty(context: &mut BTreeMap<String, String>, key: &str, value: String) {
    if !value.trim().is_empty() {
        context.insert(key.to_string(), value);
    }
}

pub(crate) fn malformed(
    path: &Utf8Path,
    format: &'static str,
    message: impl ToString,
) -> AdaptError {
    AdaptError::Malformed {
        path: path.to_owned(),
        format,
        message: message.to_string(),
    }
}

fn canonical_game_context(value: &str) -> Option<String> {
    let normalized = value
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match normalized.as_str() {
        "skyrimle" => Some("SkyrimLe".to_string()),
        "skyrimse" => Some("SkyrimSe".to_string()),
        _ => None,
    }
}
