use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::Utf8PathBuf;
use encoding_rs::{GBK, UTF_8};
use serde::{Deserialize, Serialize};
use stringer_workspace_core::{
    TranslationMeta, WorkspaceLock, claimed_entry_ids, read_translation_package_records_filtered,
    unix_ms, write_translation_package_records,
};

use crate::WorkspaceOpsError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NormalizeRuleEncoding {
    #[default]
    Auto,
    Utf8,
    Cp936,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeWorkspaceOptions {
    pub workspace: Utf8PathBuf,
    pub rules: Utf8PathBuf,
    pub file: Option<String>,
    pub apply: bool,
    pub encoding: NormalizeRuleEncoding,
    pub limit: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeWorkspaceSummary {
    pub scanned_entries: usize,
    pub changed_entries: usize,
    pub total_replacements: usize,
    pub skipped_claimed: usize,
    pub skipped_placeholder_risk: usize,
    pub warnings: Vec<NormalizeWarning>,
    pub changes: Vec<WorkspaceNormalizeChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizeWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceNormalizeChange {
    pub file: String,
    pub id: String,
    pub source: String,
    pub before: String,
    pub after: String,
    pub replacements: usize,
    pub rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub skipped_placeholder_risk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizeRule {
    id: String,
    line: usize,
    search: String,
    replace: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RuleBuilder {
    start_line: usize,
    values: BTreeMap<String, String>,
}

pub fn normalize_workspace(
    options: NormalizeWorkspaceOptions,
) -> Result<NormalizeWorkspaceSummary, WorkspaceOpsError> {
    let (rules, mut warnings) = read_rules(&options.rules, options.encoding)?;
    let _lock = if options.apply {
        Some(WorkspaceLock::acquire(&options.workspace)?)
    } else {
        None
    };
    let claimed = claimed_entry_ids(&options.workspace)?;
    let mut package =
        read_translation_package_records_filtered(&options.workspace, options.file.as_deref())?;
    let mut summary = NormalizeWorkspaceSummary {
        warnings: std::mem::take(&mut warnings),
        ..NormalizeWorkspaceSummary::default()
    };

    for file in &mut package.files {
        for record in &mut file.records {
            if claimed.contains(&record.id) {
                if record.translation.is_some() {
                    summary.skipped_claimed += 1;
                }
                continue;
            }
            let Some(before) = record.translation.clone() else {
                continue;
            };
            summary.scanned_entries += 1;

            let outcome = normalize_text(&before, &rules);
            if outcome.replacements == 0 {
                continue;
            }

            let placeholder_risk = placeholder_tokens(&before) != placeholder_tokens(&outcome.text);
            if placeholder_risk {
                summary.skipped_placeholder_risk += 1;
                push_change(
                    &mut summary,
                    options.limit,
                    WorkspaceNormalizeChange {
                        file: file.manifest_file.path.clone(),
                        id: record.id.clone(),
                        source: record.source.clone(),
                        before,
                        after: outcome.text,
                        replacements: outcome.replacements,
                        rule_ids: outcome.rule_ids,
                        skipped_placeholder_risk: true,
                    },
                );
                continue;
            }

            summary.changed_entries += 1;
            summary.total_replacements += outcome.replacements;
            push_change(
                &mut summary,
                options.limit,
                WorkspaceNormalizeChange {
                    file: file.manifest_file.path.clone(),
                    id: record.id.clone(),
                    source: record.source.clone(),
                    before,
                    after: outcome.text.clone(),
                    replacements: outcome.replacements,
                    rule_ids: outcome.rule_ids,
                    skipped_placeholder_risk: false,
                },
            );
            if options.apply {
                record.translation = Some(outcome.text);
                update_translation_meta(&mut record.translation_meta);
            }
        }
    }

    if options.apply && summary.changed_entries > 0 {
        write_translation_package_records(&options.workspace, &package)?;
    }
    Ok(summary)
}

fn read_rules(
    path: &Utf8PathBuf,
    encoding: NormalizeRuleEncoding,
) -> Result<(Vec<NormalizeRule>, Vec<NormalizeWarning>), WorkspaceOpsError> {
    let bytes = fs::read(path).map_err(|source| {
        WorkspaceOpsError::Core(stringer_workspace_core::WorkspaceCoreError::ReadFile {
            path: path.clone(),
            source,
        })
    })?;
    let text = decode_rules(path, &bytes, encoding)?;
    parse_rules(path, text.strip_prefix('\u{feff}').unwrap_or(&text))
}

fn decode_rules(
    path: &Utf8PathBuf,
    bytes: &[u8],
    encoding: NormalizeRuleEncoding,
) -> Result<String, WorkspaceOpsError> {
    match encoding {
        NormalizeRuleEncoding::Auto => decode_utf8(bytes)
            .or_else(|| decode_cp936(bytes))
            .ok_or_else(|| WorkspaceOpsError::NormalizeRuleDecode {
                path: path.clone(),
                encoding: "utf-8 or cp936",
            }),
        NormalizeRuleEncoding::Utf8 => {
            decode_utf8(bytes).ok_or_else(|| WorkspaceOpsError::NormalizeRuleDecode {
                path: path.clone(),
                encoding: "utf-8",
            })
        }
        NormalizeRuleEncoding::Cp936 => {
            decode_cp936(bytes).ok_or_else(|| WorkspaceOpsError::NormalizeRuleDecode {
                path: path.clone(),
                encoding: "cp936",
            })
        }
    }
}

fn decode_utf8(bytes: &[u8]) -> Option<String> {
    UTF_8
        .decode_without_bom_handling_and_without_replacement(bytes)
        .map(std::borrow::Cow::into_owned)
}

fn decode_cp936(bytes: &[u8]) -> Option<String> {
    GBK.decode_without_bom_handling_and_without_replacement(bytes)
        .map(std::borrow::Cow::into_owned)
}

fn parse_rules(
    path: &Utf8PathBuf,
    text: &str,
) -> Result<(Vec<NormalizeRule>, Vec<NormalizeWarning>), WorkspaceOpsError> {
    let mut rules = Vec::new();
    let mut warnings = Vec::new();
    let mut current: Option<RuleBuilder> = None;
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if line.eq_ignore_ascii_case("StartRule") {
            if current.is_some() {
                return Err(rule_parse_error(
                    path,
                    line_number,
                    "nested StartRule is not supported",
                ));
            }
            current = Some(RuleBuilder {
                start_line: line_number,
                values: BTreeMap::new(),
            });
            continue;
        }
        if line.eq_ignore_ascii_case("EndRule") {
            let Some(builder) = current.take() else {
                return Err(rule_parse_error(
                    path,
                    line_number,
                    "EndRule without StartRule",
                ));
            };
            finish_rule(path, line_number, builder, &mut rules, &mut warnings)?;
            continue;
        }
        if let Some(builder) = &mut current
            && let Some((key, value)) = line.split_once('=')
        {
            builder
                .values
                .insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    if let Some(builder) = current {
        return Err(rule_parse_error(
            path,
            builder.start_line,
            "StartRule without EndRule",
        ));
    }
    add_cross_rule_warnings(&rules, &mut warnings);
    Ok((rules, warnings))
}

fn finish_rule(
    path: &Utf8PathBuf,
    line: usize,
    builder: RuleBuilder,
    rules: &mut Vec<NormalizeRule>,
    warnings: &mut Vec<NormalizeWarning>,
) -> Result<(), WorkspaceOpsError> {
    validate_supported_value(path, line, &builder, "mode", "0")?;
    validate_supported_value(path, line, &builder, "select", "0")?;
    validate_supported_value(path, line, &builder, "alllists", "1")?;

    let search = builder.values.get("search").cloned().unwrap_or_default();
    let replace = builder.values.get("replace").cloned().unwrap_or_default();
    if search.is_empty() || replace.is_empty() {
        warnings.push(NormalizeWarning {
            code: "rule.empty_field".to_string(),
            message: "Normalization rule has an empty Search or Replace field.".to_string(),
            line: Some(builder.start_line),
            search: Some(search.clone()),
            replace: Some(replace.clone()),
        });
    }
    if search == replace && !search.is_empty() {
        warnings.push(NormalizeWarning {
            code: "rule.same_search_replace".to_string(),
            message: "Normalization rule Search and Replace are identical.".to_string(),
            line: Some(builder.start_line),
            search: Some(search.clone()),
            replace: Some(replace.clone()),
        });
    }
    let id = format!("rule:{}", rules.len() + 1);
    rules.push(NormalizeRule {
        id,
        line: builder.start_line,
        search,
        replace,
    });
    Ok(())
}

fn validate_supported_value(
    path: &Utf8PathBuf,
    line: usize,
    builder: &RuleBuilder,
    key: &'static str,
    expected: &'static str,
) -> Result<(), WorkspaceOpsError> {
    let Some(value) = builder.values.get(key) else {
        return Ok(());
    };
    if value == expected {
        return Ok(());
    }
    Err(rule_parse_error(
        path,
        line,
        format!("unsupported {key}={value}; only {key}={expected} is supported"),
    ))
}

fn add_cross_rule_warnings(rules: &[NormalizeRule], warnings: &mut Vec<NormalizeWarning>) {
    let mut targets = BTreeMap::<&str, BTreeSet<&str>>::new();
    for rule in rules {
        if rule.search.chars().count() <= 4 && !rule.search.is_empty() {
            warnings.push(NormalizeWarning {
                code: "rule.short_search".to_string(),
                message: "Normalization rule Search is 4 characters or shorter.".to_string(),
                line: Some(rule.line),
                search: Some(rule.search.clone()),
                replace: Some(rule.replace.clone()),
            });
        }
        targets
            .entry(&rule.search)
            .or_default()
            .insert(&rule.replace);
    }
    for (search, replacements) in targets {
        if replacements.len() > 1 {
            warnings.push(NormalizeWarning {
                code: "rule.duplicate_search_conflict".to_string(),
                message: "Multiple normalization rules use the same Search with different Replace values.".to_string(),
                line: None,
                search: Some(search.to_string()),
                replace: Some(
                    replacements
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                        .join(" | "),
                ),
            });
        }
    }
}

fn rule_parse_error(
    path: &Utf8PathBuf,
    line: usize,
    message: impl Into<String>,
) -> WorkspaceOpsError {
    WorkspaceOpsError::NormalizeRuleParse {
        path: path.clone(),
        line,
        message: message.into(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct NormalizeTextOutcome {
    text: String,
    replacements: usize,
    rule_ids: Vec<String>,
}

fn normalize_text(text: &str, rules: &[NormalizeRule]) -> NormalizeTextOutcome {
    let mut current = text.to_string();
    let mut replacements = 0usize;
    let mut rule_ids = Vec::new();
    for rule in rules {
        if rule.search.is_empty() || rule.search == rule.replace {
            continue;
        }
        let Some((updated, count)) = replace_literal(&current, &rule.search, &rule.replace) else {
            continue;
        };
        current = updated;
        replacements += count;
        rule_ids.push(rule.id.clone());
    }
    NormalizeTextOutcome {
        text: current,
        replacements,
        rule_ids,
    }
}

fn replace_literal(haystack: &str, needle: &str, replacement: &str) -> Option<(String, usize)> {
    if needle.is_empty() {
        return None;
    }
    let mut matches = haystack.match_indices(needle).peekable();
    matches.peek()?;

    let mut last = 0usize;
    let mut count = 0usize;
    let mut output = String::with_capacity(haystack.len());
    for (index, matched) in matches {
        output.push_str(&haystack[last..index]);
        output.push_str(replacement);
        last = index + matched.len();
        count += 1;
    }
    output.push_str(&haystack[last..]);
    Some((output, count))
}

fn placeholder_tokens(text: &str) -> BTreeMap<String, usize> {
    let mut tokens = BTreeMap::new();
    collect_delimited_tokens(text, '<', '>', &mut tokens);
    collect_delimited_tokens(text, '{', '}', &mut tokens);
    collect_delimited_tokens(text, '%', '%', &mut tokens);
    collect_printf_tokens(text, &mut tokens);
    tokens
}

fn collect_delimited_tokens(
    text: &str,
    open: char,
    close: char,
    tokens: &mut BTreeMap<String, usize>,
) {
    let mut start: Option<usize> = None;
    for (index, ch) in text.char_indices() {
        if start.is_none() && ch == open {
            start = Some(index);
            continue;
        }
        if let Some(open_index) = start
            && ch == close
            && index > open_index
        {
            let end = index + ch.len_utf8();
            *tokens.entry(text[open_index..end].to_string()).or_default() += 1;
            start = None;
        }
    }
}

fn collect_printf_tokens(text: &str, tokens: &mut BTreeMap<String, usize>) {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index + 1 < chars.len() {
        if chars[index] == '%' && matches!(chars[index + 1], 's' | 'd' | 'f' | 'i') {
            let token = chars[index..=index + 1].iter().collect::<String>();
            *tokens.entry(token).or_default() += 1;
            index += 2;
            continue;
        }
        index += 1;
    }
}

fn update_translation_meta(meta: &mut Option<TranslationMeta>) {
    let now = unix_ms();
    match meta {
        Some(meta) => meta.updated_at_unix_ms = Some(now),
        None => {
            *meta = Some(TranslationMeta {
                origin: Some("normalized".to_string()),
                updated_at_unix_ms: Some(now),
                skip_reason: None,
            });
        }
    }
}

fn push_change(
    summary: &mut NormalizeWorkspaceSummary,
    limit: usize,
    change: WorkspaceNormalizeChange,
) {
    if summary.changes.len() < limit {
        summary.changes.push(change);
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
