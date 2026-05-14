use std::cmp::Ordering;
use std::collections::BTreeMap;

use regex::{Regex, RegexBuilder};
use serde::Serialize;
use stringer_pipeline::{KnowledgeBase, MemoryQuality, PipelineDiagnostic, TermStatus};

use crate::WorkspaceError;

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

pub(crate) fn search_knowledge(
    knowledge: &KnowledgeBase,
    options: &KnowledgeSearchOptions<'_>,
) -> Result<KnowledgeSearchOutput, WorkspaceError> {
    let regex = lookup_regex(options)?;
    let mut ranked = Vec::new();
    if options.source != LookupKnowledgeSource::Terms {
        collect_memory_results(knowledge, options, regex.as_ref(), &mut ranked);
    }
    if options.source != LookupKnowledgeSource::Memory {
        collect_term_results(knowledge, options, regex.as_ref(), &mut ranked);
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

fn collect_memory_results(
    knowledge: &KnowledgeBase,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
    ranked: &mut Vec<RankedResult>,
) {
    for item in knowledge.memory() {
        if item.quality() == MemoryQuality::Rejected {
            continue;
        }
        if item.source_locale() != options.source_locale
            || item.target_locale() != options.target_locale
        {
            continue;
        }
        let Some(field_match) = best_field_match(item.source(), item.target(), options, regex)
        else {
            continue;
        };
        let context = context_match(options.context, item.context());
        ranked.push(RankedResult {
            layer_priority: layer_priority(item.layer()),
            quality_priority: memory_quality_priority(item.quality()),
            source_len: item.source().chars().count(),
            context_matches: context.matches,
            context_conflicts: context.conflicts,
            result: KnowledgeLookupResult {
                kind: "memory".to_string(),
                id: item.id().to_string(),
                layer: item.layer().to_string(),
                source: item.source().to_string(),
                target: item.target().to_string(),
                match_field: field_match.field.to_string(),
                match_kind: field_match.kind.to_string(),
                score: field_match.score,
                quality: Some(item.quality().as_str().to_string()),
                status: None,
            },
        });
    }
}

fn collect_term_results(
    knowledge: &KnowledgeBase,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
    ranked: &mut Vec<RankedResult>,
) {
    for term in knowledge.terms() {
        let Some(field_match) = best_field_match(term.source(), term.target(), options, regex)
        else {
            continue;
        };
        let context = scope_match(options.context, term.scope_values());
        ranked.push(RankedResult {
            layer_priority: layer_priority(term.layer()),
            quality_priority: term_status_priority(term.status()),
            source_len: term.source().chars().count(),
            context_matches: context.matches,
            context_conflicts: context.conflicts,
            result: KnowledgeLookupResult {
                kind: "term".to_string(),
                id: term.id().to_string(),
                layer: term.layer().to_string(),
                source: term.source().to_string(),
                target: term.target().to_string(),
                match_field: field_match.field.to_string(),
                match_kind: field_match.kind.to_string(),
                score: field_match.score,
                quality: None,
                status: Some(term.status().as_str().to_string()),
            },
        });
    }
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

fn lookup_regex(options: &KnowledgeSearchOptions<'_>) -> Result<Option<Regex>, WorkspaceError> {
    if options.mode != LookupKnowledgeMode::Regex {
        return Ok(None);
    }
    RegexBuilder::new(options.query)
        .case_insensitive(!options.case_sensitive)
        .build()
        .map(Some)
        .map_err(|source| WorkspaceError::InvalidLookupRegex {
            pattern: options.query.to_string(),
            source,
        })
}

fn best_field_match(
    source: &str,
    target: &str,
    options: &KnowledgeSearchOptions<'_>,
    regex: Option<&Regex>,
) -> Option<FieldMatch> {
    let mut candidates = Vec::new();
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Source
    ) && let Some(candidate) = text_match(source, "source", options, regex)
    {
        candidates.push(candidate);
    }
    if matches!(
        options.field,
        LookupKnowledgeField::Both | LookupKnowledgeField::Target
    ) && let Some(candidate) = text_match(target, "target", options, regex)
    {
        candidates.push(candidate);
    }
    candidates
        .into_iter()
        .max_by_key(|candidate| candidate.score)
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

fn context_match(
    actual: &BTreeMap<String, String>,
    expected: &BTreeMap<String, String>,
) -> ContextMatch {
    let mut result = ContextMatch::default();
    for (key, value) in expected {
        if let Some(actual) = actual.get(key) {
            if actual == value {
                result.matches += 1;
            } else {
                result.conflicts += 1;
            }
        }
    }
    result
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
    if field == "source" { 60_000 } else { 50_000 }
}

fn prefix_score(field: &str) -> i32 {
    if field == "source" { 40_000 } else { 30_000 }
}

fn contains_score(field: &str) -> i32 {
    if field == "source" { 20_000 } else { 10_000 }
}

fn regex_score(field: &str) -> i32 {
    if field == "source" { 9_000 } else { 8_000 }
}

fn layer_priority(layer: &str) -> usize {
    match layer {
        "built-in" => 0,
        "global" => 1,
        "library" => 2,
        "project" => 3,
        "override" => 4,
        _ => 0,
    }
}

fn memory_quality_priority(quality: MemoryQuality) -> usize {
    match quality {
        MemoryQuality::Confirmed => 4,
        MemoryQuality::Imported => 3,
        MemoryQuality::Machine => 2,
        MemoryQuality::Rejected => 1,
    }
}

fn term_status_priority(status: TermStatus) -> usize {
    match status {
        TermStatus::Preferred => 3,
        TermStatus::Allowed => 2,
        TermStatus::Forbidden => 1,
    }
}
