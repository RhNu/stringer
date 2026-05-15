use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use regex::{Regex, RegexBuilder};
use rusqlite::{Connection, params, params_from_iter, types::Value};
use serde::Serialize;
use stringer_pipeline::PipelineDiagnostic;

use crate::KnowledgeError;
use crate::index::{IndexedKnowledgeId, normalize_lookup_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LookupKnowledgeMode {
    Contains,
    Regex,
}

impl LookupKnowledgeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Regex => "regex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupKnowledgeSource {
    All,
    Memory,
    Terms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupKnowledgeField {
    Both,
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeLookup {
    pub query: String,
    pub mode: LookupKnowledgeMode,
    pub total_matches: usize,
    pub results: Vec<KnowledgeLookupResult>,
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub index_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct KnowledgeLookupResult {
    pub kind: String,
    pub id: String,
    pub layer: String,
    pub source: String,
    pub target: String,
    pub match_field: String,
    pub match_kind: String,
    pub score: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

pub(crate) struct KnowledgeSearchOptions<'a> {
    pub query: &'a str,
    pub mode: LookupKnowledgeMode,
    pub source: LookupKnowledgeSource,
    pub field: LookupKnowledgeField,
    pub limit: usize,
    pub case_sensitive: bool,
    pub source_locale: &'a str,
    pub target_locale: &'a str,
    pub context: &'a BTreeMap<String, String>,
}

pub(crate) struct KnowledgeSearchOutput {
    pub total_matches: usize,
    pub results: Vec<KnowledgeLookupResult>,
}

pub(crate) fn search_knowledge_indexes(
    paths: &[Utf8PathBuf],
    options: &KnowledgeSearchOptions<'_>,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<KnowledgeSearchOutput, KnowledgeError> {
    search_ranked_knowledge_indexes(paths, options, suppressed_items)
}

fn search_ranked_knowledge_indexes(
    paths: &[Utf8PathBuf],
    options: &KnowledgeSearchOptions<'_>,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<KnowledgeSearchOutput, KnowledgeError> {
    let regex = lookup_regex(options)?;
    let mut ranked = Vec::new();
    for path in paths {
        ranked.extend(ranked_results_for_index(
            path,
            options,
            regex.as_ref(),
            suppressed_items,
        )?);
    }
    ranked.sort_by(compare_ranked_results);

    let total_matches = ranked.len();
    let results = ranked
        .into_iter()
        .take(options.limit)
        .map(|ranked| ranked.result)
        .collect();
    Ok(KnowledgeSearchOutput {
        total_matches,
        results,
    })
}

fn ranked_results_for_index(
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<RankedResult>, KnowledgeError> {
    let connection = Connection::open(path).map_err(|source| KnowledgeError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    let rows = candidate_rows(&connection, path, options)?;
    let mut ranked = Vec::new();
    for mut row in rows {
        if suppressed_items.contains(&IndexedKnowledgeId {
            kind: row.item_kind.clone(),
            id: row.id.clone(),
            layer: row.layer.clone(),
        }) {
            continue;
        }
        row.aliases = aliases_for_row(&connection, path, row.rowid)?;
        row.scope = scope_for_row(&connection, path, row.rowid)?;
        let Some(field_match) = best_index_row_match(&row, options, regex) else {
            continue;
        };
        let context = scope_match(options.context, &row.scope);
        if context.conflicts > 0 {
            continue;
        }
        ranked.push(RankedResult {
            layer_priority: layer_priority(&row.layer),
            quality_priority: index_quality_priority(
                row.item_kind.as_str(),
                row.quality.as_deref(),
                row.status.as_deref(),
            ),
            source_len: row.source_len,
            context_matches: context.matches,
            context_conflicts: context.conflicts,
            result: KnowledgeLookupResult {
                kind: row.item_kind,
                id: row.id,
                layer: row.layer,
                source: row.source,
                target: row.target,
                match_field: field_match.field.to_string(),
                match_kind: field_match.kind.to_string(),
                score: field_match.score,
                quality: row.quality,
                status: row.status,
            },
        });
    }
    Ok(ranked)
}

#[derive(Debug, Clone)]
struct IndexRow {
    rowid: i64,
    item_kind: String,
    id: String,
    layer: String,
    source: String,
    target: String,
    quality: Option<String>,
    status: Option<String>,
    source_len: usize,
    aliases: Vec<String>,
    scope: BTreeMap<String, Vec<String>>,
}

fn candidate_rows(
    connection: &Connection,
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
) -> Result<Vec<IndexRow>, KnowledgeError> {
    if options.query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let query = normalize_lookup_text(options.query);
    let rowids = if options.mode == LookupKnowledgeMode::Regex || query.chars().count() < 3 {
        None
    } else {
        let mut rowids = exact_prefix_candidate_rowids(connection, path, options, &query)?;
        rowids.extend(fts_candidate_rowids(connection, path, &query)?);
        Some(rowids)
    };
    select_candidate_rows(connection, path, options, rowids.as_ref())
}

fn exact_prefix_candidate_rowids(
    connection: &Connection,
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
    query: &str,
) -> Result<BTreeSet<i64>, KnowledgeError> {
    let prefix = format!("{query}%");
    let mut rowids = BTreeSet::new();
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Source
    ) {
        rowids.extend(rowids_for_text_column(
            connection,
            path,
            "source_norm",
            query,
            &prefix,
        )?);
        rowids.extend(rowids_for_aliases(connection, path, query, &prefix)?);
    }
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Target
    ) {
        rowids.extend(rowids_for_text_column(
            connection,
            path,
            "target_norm",
            query,
            &prefix,
        )?);
    }
    Ok(rowids)
}

fn rowids_for_text_column(
    connection: &Connection,
    path: &Utf8Path,
    column: &str,
    query: &str,
    prefix: &str,
) -> Result<BTreeSet<i64>, KnowledgeError> {
    let sql = format!("SELECT rowid FROM items WHERE {column} = ?1 OR {column} LIKE ?2");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![query, prefix], |row| row.get::<_, i64>(0))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    collect_rowids(path, rows)
}

fn rowids_for_aliases(
    connection: &Connection,
    path: &Utf8Path,
    query: &str,
    prefix: &str,
) -> Result<BTreeSet<i64>, KnowledgeError> {
    let mut statement = connection
        .prepare("SELECT item_rowid FROM aliases WHERE alias_norm = ?1 OR alias_norm LIKE ?2")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![query, prefix], |row| row.get::<_, i64>(0))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    collect_rowids(path, rows)
}

fn fts_candidate_rowids(
    connection: &Connection,
    path: &Utf8Path,
    query: &str,
) -> Result<BTreeSet<i64>, KnowledgeError> {
    let query = fts_phrase(query);
    let mut statement = connection
        .prepare("SELECT rowid FROM items_fts WHERE items_fts MATCH ?1")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![query], |row| row.get::<_, i64>(0))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    collect_rowids(path, rows)
}

fn collect_rowids(
    path: &Utf8Path,
    rows: impl Iterator<Item = rusqlite::Result<i64>>,
) -> Result<BTreeSet<i64>, KnowledgeError> {
    let mut rowids = BTreeSet::new();
    for row in rows {
        rowids.insert(row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(rowids)
}

fn select_candidate_rows(
    connection: &Connection,
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
    rowids: Option<&BTreeSet<i64>>,
) -> Result<Vec<IndexRow>, KnowledgeError> {
    match rowids {
        Some(rowids) if rowids.is_empty() => Ok(Vec::new()),
        Some(rowids) => select_candidate_rows_by_rowid(connection, path, options, rowids),
        None => select_candidate_rows_by_filter(connection, path, options),
    }
}

fn select_candidate_rows_by_filter(
    connection: &Connection,
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
) -> Result<Vec<IndexRow>, KnowledgeError> {
    let mut statement = connection
        .prepare(&candidate_rows_sql(None))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let source_filter = match options.source {
        LookupKnowledgeSource::All => "all",
        LookupKnowledgeSource::Memory => "memory",
        LookupKnowledgeSource::Terms => "term",
    };
    let rows = statement
        .query_map(
            params![
                source_filter,
                source_filter,
                options.source_locale,
                options.target_locale,
            ],
            index_row_from_sql,
        )
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    collect_index_rows(path, rows)
}

fn select_candidate_rows_by_rowid(
    connection: &Connection,
    path: &Utf8Path,
    options: &KnowledgeSearchOptions<'_>,
    rowids: &BTreeSet<i64>,
) -> Result<Vec<IndexRow>, KnowledgeError> {
    let source_filter = match options.source {
        LookupKnowledgeSource::All => "all",
        LookupKnowledgeSource::Memory => "memory",
        LookupKnowledgeSource::Terms => "term",
    };
    let mut candidates = Vec::new();
    for chunk in rowids.iter().copied().collect::<Vec<_>>().chunks(500) {
        let mut values = chunk
            .iter()
            .map(|rowid| Value::Integer(*rowid))
            .collect::<Vec<_>>();
        values.extend([
            Value::Text(source_filter.to_string()),
            Value::Text(source_filter.to_string()),
            Value::Text(options.source_locale.to_string()),
            Value::Text(options.target_locale.to_string()),
        ]);
        let mut statement = connection
            .prepare(&candidate_rows_sql(Some(chunk.len())))
            .map_err(|source| KnowledgeError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        let rows = statement
            .query_map(params_from_iter(values), index_row_from_sql)
            .map_err(|source| KnowledgeError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        candidates.extend(collect_index_rows(path, rows)?);
    }
    Ok(candidates)
}

fn candidate_rows_sql(rowid_count: Option<usize>) -> String {
    let rowid_filter = rowid_count.map(|count| {
        format!(
            "rowid IN ({}) AND ",
            (0..count).map(|_| "?").collect::<Vec<_>>().join(", ")
        )
    });
    format!(
        concat!(
            "SELECT rowid, item_kind, id, layer, source, target, quality, status, source_len ",
            "FROM items WHERE ",
            "{}",
            "(? = 'all' OR item_kind = ?) AND ",
            "(item_kind != 'memory' OR (source_locale = ? AND target_locale = ? AND quality != 'rejected'))"
        ),
        rowid_filter.unwrap_or_default()
    )
}

fn collect_index_rows(
    path: &Utf8Path,
    rows: impl Iterator<Item = rusqlite::Result<IndexRow>>,
) -> Result<Vec<IndexRow>, KnowledgeError> {
    let mut candidates = Vec::new();
    for row in rows {
        candidates.push(row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(candidates)
}

fn index_row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexRow> {
    Ok(IndexRow {
        rowid: row.get::<_, i64>(0)?,
        item_kind: row.get::<_, String>(1)?,
        id: row.get::<_, String>(2)?,
        layer: row.get::<_, String>(3)?,
        source: row.get::<_, String>(4)?,
        target: row.get::<_, String>(5)?,
        quality: row.get::<_, Option<String>>(6)?,
        status: row.get::<_, Option<String>>(7)?,
        source_len: row.get::<_, i64>(8)? as usize,
        aliases: Vec::new(),
        scope: BTreeMap::new(),
    })
}

fn aliases_for_row(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<Vec<String>, KnowledgeError> {
    let mut statement = connection
        .prepare("SELECT alias FROM aliases WHERE item_rowid = ?1")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![rowid], |row| row.get::<_, String>(0))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut aliases = Vec::new();
    for row in rows {
        aliases.push(row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(aliases)
}

fn scope_for_row(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<BTreeMap<String, Vec<String>>, KnowledgeError> {
    let mut statement = connection
        .prepare("SELECT key, value FROM item_scope WHERE item_rowid = ?1")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![rowid], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut scope = BTreeMap::<String, Vec<String>>::new();
    for row in rows {
        let (key, value) = row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
        scope.entry(key).or_default().push(value);
    }
    Ok(scope)
}

fn best_index_row_match(
    row: &IndexRow,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
) -> Option<FieldMatch> {
    let mut candidates = Vec::new();
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Source
    ) {
        if let Some(candidate) = text_match(&row.source, "source", options, regex) {
            candidates.push(candidate);
        }
        for alias in &row.aliases {
            if let Some(candidate) = text_match(alias, "alias", options, regex) {
                candidates.push(candidate);
            }
        }
    }
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Target
    ) && let Some(candidate) = text_match(&row.target, "target", options, regex)
    {
        candidates.push(candidate);
    }
    candidates
        .into_iter()
        .max_by_key(|candidate| candidate.score)
}

#[derive(Debug, Clone)]
struct RankedResult {
    result: KnowledgeLookupResult,
    context_matches: usize,
    context_conflicts: usize,
    layer_priority: usize,
    quality_priority: usize,
    source_len: usize,
}

#[derive(Debug, Clone, Copy)]
struct FieldMatch {
    field: &'static str,
    kind: &'static str,
    score: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct ContextMatch {
    matches: usize,
    conflicts: usize,
}

fn lookup_regex(options: &KnowledgeSearchOptions<'_>) -> Result<Option<Regex>, KnowledgeError> {
    if options.mode != LookupKnowledgeMode::Regex {
        return Ok(None);
    }
    RegexBuilder::new(options.query)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map(Some)
        .map_err(|source| KnowledgeError::InvalidLookupRegex {
            pattern: options.query.to_string(),
            source,
        })
}

fn text_match(
    value: &str,
    field: &'static str,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
) -> Option<FieldMatch> {
    let query_norm = normalize_exact(options.query, options.case_sensitive);
    if !query_norm.is_empty() && normalize_exact(value, options.case_sensitive) == query_norm {
        return Some(FieldMatch {
            field,
            kind: "exact",
            score: exact_score(field),
        });
    }

    match options.mode {
        LookupKnowledgeMode::Contains => contains_text_match(value, field, options),
        LookupKnowledgeMode::Regex => regex.and_then(|regex| {
            regex.is_match(value).then_some(FieldMatch {
                field,
                kind: "regex",
                score: regex_score(field),
            })
        }),
    }
}

fn contains_text_match(
    value: &str,
    field: &'static str,
    options: &KnowledgeSearchOptions<'_>,
) -> Option<FieldMatch> {
    let query = comparable(options.query, options.case_sensitive);
    if query.is_empty() {
        return None;
    }
    let value = comparable(value, options.case_sensitive);
    if value.starts_with(&query) {
        Some(FieldMatch {
            field,
            kind: "prefix",
            score: prefix_score(field),
        })
    } else if value.contains(&query) {
        Some(FieldMatch {
            field,
            kind: "contains",
            score: contains_score(field),
        })
    } else {
        None
    }
}

fn scope_match(
    actual: &BTreeMap<String, String>,
    expected: &BTreeMap<String, Vec<String>>,
) -> ContextMatch {
    let mut result = ContextMatch::default();
    for (key, values) in expected {
        if let Some(actual) = actual.get(key) {
            if values.iter().any(|value| value == actual) {
                result.matches += 1;
            } else {
                result.conflicts += 1;
            }
        }
    }
    result
}

fn compare_ranked_results(left: &RankedResult, right: &RankedResult) -> Ordering {
    left.context_conflicts
        .cmp(&right.context_conflicts)
        .then(right.result.score.cmp(&left.result.score))
        .then(right.context_matches.cmp(&left.context_matches))
        .then(right.layer_priority.cmp(&left.layer_priority))
        .then(right.quality_priority.cmp(&left.quality_priority))
        .then(left.source_len.cmp(&right.source_len))
        .then(left.result.id.cmp(&right.result.id))
}

fn normalize_exact(value: &str, case_sensitive: bool) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if case_sensitive {
        normalized
    } else {
        normalized.to_lowercase()
    }
}

fn comparable(value: &str, case_sensitive: bool) -> String {
    if case_sensitive {
        value.to_string()
    } else {
        value.to_lowercase()
    }
}

fn exact_score(field: &str) -> i32 {
    match field {
        "source" => 60_000,
        "alias" => 55_000,
        _ => 50_000,
    }
}

fn prefix_score(field: &str) -> i32 {
    match field {
        "source" => 40_000,
        "alias" => 35_000,
        _ => 30_000,
    }
}

fn contains_score(field: &str) -> i32 {
    match field {
        "source" => 20_000,
        "alias" => 15_000,
        _ => 10_000,
    }
}

fn regex_score(field: &str) -> i32 {
    match field {
        "source" => 9_000,
        "alias" => 8_500,
        _ => 8_000,
    }
}

fn layer_priority(layer: &str) -> usize {
    match layer {
        "built-in" => 0,
        "global" => 1,
        "workspace" => 2,
        "override" => 3,
        _ => 0,
    }
}

fn index_quality_priority(kind: &str, quality: Option<&str>, status: Option<&str>) -> usize {
    if kind == "memory" {
        match quality {
            Some("confirmed") => 4,
            Some("imported") => 3,
            Some("machine") => 2,
            _ => 1,
        }
    } else {
        match status {
            Some("preferred") => 3,
            Some("allowed") => 2,
            Some("forbidden") => 1,
            _ => 0,
        }
    }
}

fn fts_phrase(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}
