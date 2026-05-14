use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::PipelineError;
use crate::model::{
    PipelineDiagnostic, PipelineDiagnosticSeverity, PipelineEntry, PipelineEntryKind,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeLayer {
    name: String,
    terms: Vec<Term>,
    memory: Vec<TranslationMemoryEntry>,
    rules: Vec<ReplacementRule>,
}

impl KnowledgeLayer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            terms: Vec::new(),
            memory: Vec::new(),
            rules: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_terms_toml(
        &mut self,
        path: impl Into<String>,
        text: &str,
    ) -> Result<(), PipelineError> {
        let path = path.into();
        let parsed: TermsFile =
            toml::from_str(text).map_err(|source| PipelineError::TermsToml {
                path: path.clone(),
                source,
            })?;
        let mut seen = BTreeSet::new();
        for term in parsed.terms {
            if !seen.insert(term.id.clone()) {
                return Err(PipelineError::DuplicateKnowledgeId { path, id: term.id });
            }
            self.terms.push(term.into_term(&self.name));
        }
        Ok(())
    }

    pub fn add_memory_jsonl(
        &mut self,
        path: impl Into<String>,
        text: &str,
    ) -> Result<(), PipelineError> {
        let path = path.into();
        let mut seen = BTreeSet::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let mut entry: TranslationMemoryEntry =
                serde_json::from_str(line).map_err(|source| PipelineError::MemoryJsonLine {
                    path: path.clone(),
                    line: index + 1,
                    source,
                })?;
            if !seen.insert(entry.id.clone()) {
                return Err(PipelineError::DuplicateKnowledgeId { path, id: entry.id });
            }
            entry.layer = self.name.clone();
            self.memory.push(entry);
        }
        Ok(())
    }

    pub fn add_rules_toml(
        &mut self,
        path: impl Into<String>,
        text: &str,
    ) -> Result<(), PipelineError> {
        let path = path.into();
        let parsed: RulesFile =
            toml::from_str(text).map_err(|source| PipelineError::RulesToml {
                path: path.clone(),
                source,
            })?;
        let mut seen = BTreeSet::new();
        for rule in parsed.rules {
            if !seen.insert(rule.id.clone()) {
                return Err(PipelineError::DuplicateKnowledgeId { path, id: rule.id });
            }
            self.rules.push(rule.into_rule(&self.name));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeBase {
    terms: Vec<Term>,
    memory: Vec<TranslationMemoryEntry>,
    rules: Vec<ReplacementRule>,
    merge_diagnostics: Vec<PipelineDiagnostic>,
}

impl KnowledgeBase {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_layers(layers: Vec<KnowledgeLayer>) -> Result<Self, PipelineError> {
        let mut terms = Vec::<Term>::new();
        let mut term_indexes = BTreeMap::<String, usize>::new();
        let mut rules = Vec::<ReplacementRule>::new();
        let mut rule_indexes = BTreeMap::<String, usize>::new();
        let mut memory = Vec::new();
        let mut merge_diagnostics = Vec::new();

        for layer in layers {
            for term in layer.terms {
                if let Some(index) = term_indexes.insert(term.id.clone(), terms.len()) {
                    let old = &terms[index];
                    merge_diagnostics.push(override_diagnostic(
                        "term",
                        &term.id,
                        old.layer(),
                        term.layer(),
                    ));
                    terms[index] = term;
                    term_indexes.insert(terms[index].id.clone(), index);
                } else {
                    terms.push(term);
                }
            }
            for rule in layer.rules {
                if let Some(index) = rule_indexes.insert(rule.id.clone(), rules.len()) {
                    let old = &rules[index];
                    merge_diagnostics.push(override_diagnostic(
                        "replacement rule",
                        &rule.id,
                        old.layer(),
                        rule.layer(),
                    ));
                    rules[index] = rule;
                    rule_indexes.insert(rules[index].id.clone(), index);
                } else {
                    rules.push(rule);
                }
            }
            memory.extend(layer.memory);
        }
        for rule in &rules {
            if rule.mode() == RuleMode::Regex
                && let Err(error) = regex::Regex::new(rule.pattern())
            {
                merge_diagnostics.push(invalid_regex_diagnostic(rule, error.to_string()));
            }
        }

        Ok(Self {
            terms,
            memory,
            rules,
            merge_diagnostics,
        })
    }

    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    pub fn memory(&self) -> &[TranslationMemoryEntry] {
        &self.memory
    }

    pub fn rules(&self) -> &[ReplacementRule] {
        &self.rules
    }

    pub fn merge_diagnostics(&self) -> &[PipelineDiagnostic] {
        &self.merge_diagnostics
    }

    pub fn add_diagnostic(&mut self, diagnostic: PipelineDiagnostic) {
        self.merge_diagnostics.push(diagnostic);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    id: String,
    source: String,
    target: String,
    aliases: Vec<String>,
    case_sensitive: bool,
    status: TermStatus,
    scope: Scope,
    tags: Vec<String>,
    note: Option<String>,
    layer: String,
}

impl Term {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn status(&self) -> TermStatus {
        self.status
    }

    pub fn scope_values(&self) -> &BTreeMap<String, Vec<String>> {
        self.scope.values()
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn layer(&self) -> &str {
        &self.layer
    }

    pub fn matches_entry(&self, entry: &PipelineEntry) -> Option<&'static str> {
        if !self.scope.matches(entry) {
            return None;
        }
        if contains_text(entry.source_text(), &self.source, self.case_sensitive) {
            return Some("source");
        }
        if self
            .aliases
            .iter()
            .any(|alias| contains_text(entry.source_text(), alias, self.case_sensitive))
        {
            return Some("alias");
        }
        None
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TermStatus {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

impl TermStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preferred => "preferred",
            Self::Allowed => "allowed",
            Self::Forbidden => "forbidden",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TranslationMemoryEntry {
    id: String,
    source: String,
    target: String,
    source_locale: String,
    target_locale: String,
    #[serde(default)]
    context: BTreeMap<String, String>,
    #[serde(default)]
    origin: serde_json::Value,
    #[serde(default)]
    quality: MemoryQuality,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(skip)]
    layer: String,
}

impl TranslationMemoryEntry {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn source_locale(&self) -> &str {
        &self.source_locale
    }

    pub fn target_locale(&self) -> &str {
        &self.target_locale
    }

    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    pub fn quality(&self) -> MemoryQuality {
        self.quality
    }

    pub fn origin(&self) -> &serde_json::Value {
        &self.origin
    }

    pub fn created_at(&self) -> Option<&str> {
        self.created_at.as_deref()
    }

    pub fn updated_at(&self) -> Option<&str> {
        self.updated_at.as_deref()
    }

    pub fn layer(&self) -> &str {
        &self.layer
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryQuality {
    Confirmed,
    #[default]
    Imported,
    Machine,
    Rejected,
}

impl MemoryQuality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Imported => "imported",
            Self::Machine => "machine",
            Self::Rejected => "rejected",
        }
    }

    pub fn can_auto_fill(self) -> bool {
        matches!(self, Self::Confirmed | Self::Imported)
    }

    pub fn can_suggest(self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementRule {
    id: String,
    stage: RuleStage,
    mode: RuleMode,
    pattern: String,
    replacement: String,
    enabled: bool,
    scope: Scope,
    note: Option<String>,
    layer: String,
}

impl ReplacementRule {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn stage(&self) -> RuleStage {
        self.stage
    }

    pub fn mode(&self) -> RuleMode {
        self.mode
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn replacement(&self) -> &str {
        &self.replacement
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn scope_values(&self) -> &BTreeMap<String, Vec<String>> {
        self.scope.values()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn layer(&self) -> &str {
        &self.layer
    }

    pub fn matches_entry(&self, entry: &PipelineEntry) -> bool {
        if !self.enabled || !self.scope.matches(entry) {
            return false;
        }
        match self.mode {
            RuleMode::Literal => entry.source_text().contains(&self.pattern),
            RuleMode::Regex => regex::Regex::new(&self.pattern)
                .is_ok_and(|pattern| pattern.is_match(entry.source_text())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStage {
    PreTranslate,
    PostTranslate,
}

impl RuleStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreTranslate => "pre_translate",
            Self::PostTranslate => "post_translate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleMode {
    Literal,
    Regex,
}

impl RuleMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Regex => "regex",
        }
    }
}

#[derive(Debug, Deserialize)]
struct TermsFile {
    #[serde(default)]
    terms: Vec<TermDefinition>,
}

#[derive(Debug, Deserialize)]
struct TermDefinition {
    id: String,
    source: String,
    target: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    case_sensitive: bool,
    #[serde(default)]
    status: TermStatus,
    #[serde(default)]
    scope: ScopeDefinition,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    note: Option<String>,
}

impl TermDefinition {
    fn into_term(self, layer: &str) -> Term {
        Term {
            id: self.id,
            source: self.source,
            target: self.target,
            aliases: self.aliases,
            case_sensitive: self.case_sensitive,
            status: self.status,
            scope: self.scope.into_scope(),
            tags: self.tags,
            note: self.note,
            layer: layer.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(default)]
    rules: Vec<RuleDefinition>,
}

#[derive(Debug, Deserialize)]
struct RuleDefinition {
    id: String,
    stage: RuleStage,
    mode: RuleMode,
    pattern: String,
    replacement: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    scope: ScopeDefinition,
    #[serde(default)]
    note: Option<String>,
}

impl RuleDefinition {
    fn into_rule(self, layer: &str) -> ReplacementRule {
        ReplacementRule {
            id: self.id,
            stage: self.stage,
            mode: self.mode,
            pattern: self.pattern,
            replacement: self.replacement,
            enabled: self.enabled,
            scope: self.scope.into_scope(),
            note: self.note,
            layer: layer.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Scope {
    values: BTreeMap<String, Vec<String>>,
}

impl Scope {
    fn matches(&self, entry: &PipelineEntry) -> bool {
        self.values.iter().all(|(key, expected)| {
            entry
                .entry_value(key)
                .is_some_and(|actual| expected.iter().any(|value| value == actual))
        })
    }

    fn values(&self) -> &BTreeMap<String, Vec<String>> {
        &self.values
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ScopeDefinition {
    #[serde(default)]
    game: Option<OneOrMany>,
    #[serde(default)]
    source_locale: Option<OneOrMany>,
    #[serde(default)]
    target_locale: Option<OneOrMany>,
    #[serde(default)]
    kind: Option<OneOrMany>,
    #[serde(default)]
    record_type: Option<OneOrMany>,
    #[serde(default)]
    asset_path: Option<OneOrMany>,
}

impl ScopeDefinition {
    fn into_scope(self) -> Scope {
        let mut values = BTreeMap::new();
        insert_scope(&mut values, "game", self.game);
        insert_scope(&mut values, "source_locale", self.source_locale);
        insert_scope(&mut values, "target_locale", self.target_locale);
        insert_scope(&mut values, "kind", self.kind);
        insert_scope(&mut values, "record_type", self.record_type);
        insert_scope(&mut values, "asset_path", self.asset_path);
        Scope { values }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => vec![value],
            Self::Many(values) => values,
        }
    }
}

fn insert_scope(values: &mut BTreeMap<String, Vec<String>>, key: &str, value: Option<OneOrMany>) {
    if let Some(value) = value {
        values.insert(key.to_string(), value.into_vec());
    }
}

fn contains_text(haystack: &str, needle: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        haystack.contains(needle)
    } else {
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }
}

fn override_diagnostic(
    item: &str,
    id: &str,
    old_layer: &str,
    new_layer: &str,
) -> PipelineDiagnostic {
    PipelineDiagnostic::new(
        PipelineDiagnosticSeverity::Warning,
        "knowledge.override",
        format!("{new_layer} {item} `{id}` overrides {old_layer} {item} `{id}`"),
        "",
    )
    .with_layer(new_layer)
    .with_rule_id(id)
}

fn invalid_regex_diagnostic(rule: &ReplacementRule, error: String) -> PipelineDiagnostic {
    PipelineDiagnostic::new(
        PipelineDiagnosticSeverity::Warning,
        "replacement.regex_invalid",
        format!(
            "Replacement rule `{}` has an invalid regex: {error}",
            rule.id()
        ),
        "",
    )
    .with_layer(rule.layer())
    .with_rule_id(rule.id())
}

pub(crate) fn stage_matches_rule(stage: crate::PipelineStage, rule: &ReplacementRule) -> bool {
    matches!(
        (stage, rule.stage()),
        (crate::PipelineStage::PreTranslate, RuleStage::PreTranslate)
            | (
                crate::PipelineStage::PostTranslate,
                RuleStage::PostTranslate
            )
    )
}

pub(crate) fn supported_knowledge_kind(kind: PipelineEntryKind) -> bool {
    matches!(
        kind,
        PipelineEntryKind::Plugin | PipelineEntryKind::Strings | PipelineEntryKind::Scaleform
    )
}
