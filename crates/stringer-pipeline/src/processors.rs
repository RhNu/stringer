use std::collections::BTreeMap;

use serde_json::json;

use crate::knowledge::{
    KnowledgeBase, MemoryQuality, ReplacementRule, Term, TermStatus, TranslationMemoryEntry,
    stage_matches_rule, supported_knowledge_kind,
};
use crate::model::{
    PipelineAnnotation, PipelineDiagnostic, PipelineDiagnosticSeverity, PipelineEntry,
    PipelineOptions, PipelineReport, PipelineStage, annotation_payload,
};

pub trait PipelineProcessor {
    fn name(&self) -> &'static str;

    fn supports_entry(&self, _entry: &PipelineEntry) -> bool {
        true
    }

    fn can_mutate_translation(&self) -> bool {
        false
    }

    fn process(
        &self,
        stage: PipelineStage,
        entry: &mut PipelineEntry,
        knowledge: &KnowledgeBase,
        options: &PipelineOptions,
    );
}

pub struct Pipeline {
    processors: Vec<Box<dyn PipelineProcessor>>,
}

impl Pipeline {
    pub fn new(processors: Vec<Box<dyn PipelineProcessor>>) -> Self {
        Self { processors }
    }

    pub fn run_stage(
        &self,
        stage: PipelineStage,
        entries: &mut [PipelineEntry],
        knowledge: &KnowledgeBase,
        options: &PipelineOptions,
    ) -> PipelineReport {
        let mut report = PipelineReport {
            entries: entries.len(),
            diagnostics: knowledge.merge_diagnostics().to_vec(),
            ..PipelineReport::default()
        };
        for entry in entries {
            if !self
                .processors
                .iter()
                .any(|processor| processor.supports_entry(entry))
            {
                report.skipped += 1;
                continue;
            }
            let before_annotations = entry.annotations().len();
            let before_diagnostics = entry.diagnostics().len();
            for processor in &self.processors {
                if !processor.supports_entry(entry) {
                    continue;
                }
                let before_translated = entry.translated_text().map(str::to_string);
                processor.process(stage, entry, knowledge, options);
                if before_translated.as_deref() != entry.translated_text() {
                    if processor.can_mutate_translation() {
                        if entry.translated_text().is_some() {
                            report.auto_filled += 1;
                        }
                    } else {
                        entry.replace_translated_text(before_translated);
                        entry.add_diagnostic(
                            PipelineDiagnostic::new(
                                PipelineDiagnosticSeverity::Warning,
                                "processor.unauthorized_mutation",
                                format!(
                                    "Processor `{}` changed translated_text without mutation permission.",
                                    processor.name()
                                ),
                                entry.id(),
                            )
                            .with_rule_id(processor.name()),
                        );
                    }
                }
            }
            report.annotations += entry.annotations().len() - before_annotations;
            report
                .diagnostics
                .extend(entry.diagnostics()[before_diagnostics..].iter().cloned());
        }
        report
    }
}

#[derive(Default)]
pub struct TerminologyProcessor;

impl PipelineProcessor for TerminologyProcessor {
    fn name(&self) -> &'static str {
        "stringer.term"
    }

    fn supports_entry(&self, entry: &PipelineEntry) -> bool {
        supported_knowledge_kind(entry.kind())
    }

    fn process(
        &self,
        stage: PipelineStage,
        entry: &mut PipelineEntry,
        knowledge: &KnowledgeBase,
        _options: &PipelineOptions,
    ) {
        match stage {
            PipelineStage::Annotate => annotate_terms(entry, knowledge.terms(), self.name()),
            PipelineStage::Validate => validate_terms(entry, knowledge.terms()),
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct TranslationMemoryProcessor;

impl PipelineProcessor for TranslationMemoryProcessor {
    fn name(&self) -> &'static str {
        "stringer.memory"
    }

    fn supports_entry(&self, entry: &PipelineEntry) -> bool {
        supported_knowledge_kind(entry.kind())
    }

    fn can_mutate_translation(&self) -> bool {
        true
    }

    fn process(
        &self,
        stage: PipelineStage,
        entry: &mut PipelineEntry,
        knowledge: &KnowledgeBase,
        options: &PipelineOptions,
    ) {
        match stage {
            PipelineStage::Annotate => annotate_memory(entry, knowledge.memory(), self.name()),
            PipelineStage::MemoryApply => {
                apply_memory(entry, knowledge.memory(), options, self.name())
            }
            PipelineStage::Validate => validate_memory(entry, knowledge.memory()),
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct BasicValidationProcessor;

impl PipelineProcessor for BasicValidationProcessor {
    fn name(&self) -> &'static str {
        "stringer.validation"
    }

    fn supports_entry(&self, entry: &PipelineEntry) -> bool {
        supported_knowledge_kind(entry.kind())
    }

    fn process(
        &self,
        stage: PipelineStage,
        entry: &mut PipelineEntry,
        _knowledge: &KnowledgeBase,
        _options: &PipelineOptions,
    ) {
        if stage != PipelineStage::Validate {
            return;
        }
        validate_placeholders(entry);
        validate_scaleform_newline(entry);
        validate_empty_translation(entry);
    }
}

#[derive(Default)]
pub struct ReplacementRuleProcessor;

impl PipelineProcessor for ReplacementRuleProcessor {
    fn name(&self) -> &'static str {
        "stringer.replacement"
    }

    fn supports_entry(&self, entry: &PipelineEntry) -> bool {
        supported_knowledge_kind(entry.kind())
    }

    fn process(
        &self,
        stage: PipelineStage,
        entry: &mut PipelineEntry,
        knowledge: &KnowledgeBase,
        _options: &PipelineOptions,
    ) {
        if !matches!(
            stage,
            PipelineStage::PreTranslate | PipelineStage::PostTranslate
        ) {
            return;
        }
        let annotations = knowledge
            .rules()
            .iter()
            .filter(|rule| stage_matches_rule(stage, rule) && rule.matches_entry(entry))
            .map(|rule| rule_annotation(rule, self.name()))
            .collect::<Vec<_>>();
        for annotation in annotations {
            entry.add_annotation(annotation);
        }
    }
}

fn annotate_terms(entry: &mut PipelineEntry, terms: &[Term], processor: &str) {
    let annotations = terms
        .iter()
        .filter_map(|term| {
            term.matches_entry(entry).map(|match_kind| {
                PipelineAnnotation::new(
                    "term",
                    term.id(),
                    term.layer(),
                    1.0,
                    match_kind,
                    processor,
                    json!({
                        "source": term.source(),
                        "target": term.target(),
                        "status": term_status_name(term.status()),
                    }),
                )
            })
        })
        .collect::<Vec<_>>();
    for annotation in annotations {
        entry.add_annotation(annotation);
    }
}

fn validate_terms(entry: &mut PipelineEntry, terms: &[Term]) {
    let Some(translated) = entry.translated_text() else {
        return;
    };
    let translated = translated.to_string();
    let diagnostics = terms
        .iter()
        .filter_map(|term| {
            term.matches_entry(entry)?;
            match term.status() {
                TermStatus::Preferred if !translated.contains(term.target()) => {
                    Some(term_diagnostic(
                        entry,
                        term,
                        "term.preferred_missing",
                        format!(
                            "Expected term `{}` to use `{}`.",
                            term.source(),
                            term.target()
                        ),
                    ))
                }
                TermStatus::Forbidden if translated.contains(term.target()) => {
                    Some(term_diagnostic(
                        entry,
                        term,
                        "term.forbidden_used",
                        format!(
                            "Forbidden translation `{}` is used for `{}`.",
                            term.target(),
                            term.source()
                        ),
                    ))
                }
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    for diagnostic in diagnostics {
        entry.add_diagnostic(diagnostic);
    }
}

fn term_diagnostic(
    entry: &PipelineEntry,
    term: &Term,
    code: &'static str,
    message: String,
) -> PipelineDiagnostic {
    PipelineDiagnostic::new(
        PipelineDiagnosticSeverity::Warning,
        code,
        message,
        entry.id(),
    )
    .with_layer(term.layer())
    .with_rule_id(term.id())
}

fn annotate_memory(entry: &mut PipelineEntry, memory: &[TranslationMemoryEntry], processor: &str) {
    let annotations = memory
        .iter()
        .filter_map(|item| memory_candidate(entry, item))
        .filter(|candidate| candidate.quality.can_suggest())
        .map(|candidate| memory_annotation(&candidate, processor))
        .collect::<Vec<_>>();
    for annotation in annotations {
        entry.add_annotation(annotation);
    }
}

fn apply_memory(
    entry: &mut PipelineEntry,
    memory: &[TranslationMemoryEntry],
    options: &PipelineOptions,
    processor: &str,
) {
    if !options.allow_memory_auto_fill || entry.translated_text().is_some() {
        return;
    }
    let candidates = memory
        .iter()
        .filter_map(|item| memory_candidate(entry, item))
        .filter(|candidate| {
            candidate.quality.can_auto_fill()
                && candidate.confidence >= options.memory_auto_fill_threshold
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }

    let mut targets = BTreeMap::<String, MemoryCandidate>::new();
    for candidate in candidates {
        targets
            .entry(candidate.target.clone())
            .and_modify(|current| {
                if candidate.confidence > current.confidence {
                    *current = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    if targets.len() > 1 {
        entry.add_diagnostic(PipelineDiagnostic::new(
            PipelineDiagnosticSeverity::Warning,
            "memory.conflict",
            "Multiple high-confidence memory entries have different targets.",
            entry.id(),
        ));
        return;
    }

    let candidate = targets.into_values().next().expect("candidate");
    entry.set_translated_text(candidate.target.clone());
    entry.add_annotation(memory_annotation(&candidate, processor));
}

fn validate_memory(entry: &mut PipelineEntry, memory: &[TranslationMemoryEntry]) {
    let Some(translated) = entry.translated_text() else {
        return;
    };
    let diagnostics = memory
        .iter()
        .filter(|item| item.quality() == MemoryQuality::Rejected)
        .filter_map(|item| {
            let candidate = memory_candidate(entry, item)?;
            (candidate.target == translated).then(|| {
                PipelineDiagnostic::new(
                    PipelineDiagnosticSeverity::Warning,
                    "memory.conflict",
                    format!("Translation matches rejected memory `{}`.", item.id()),
                    entry.id(),
                )
                .with_layer(item.layer())
                .with_rule_id(item.id())
            })
        })
        .collect::<Vec<_>>();
    for diagnostic in diagnostics {
        entry.add_diagnostic(diagnostic);
    }
}

#[derive(Debug, Clone)]
struct MemoryCandidate {
    id: String,
    target: String,
    layer: String,
    confidence: f32,
    match_kind: &'static str,
    quality: MemoryQuality,
}

fn memory_candidate(
    entry: &PipelineEntry,
    item: &TranslationMemoryEntry,
) -> Option<MemoryCandidate> {
    if item.source_locale() != entry.source_locale()
        || item.target_locale() != entry.target_locale()
    {
        return None;
    }

    let (match_kind, mut confidence): (&'static str, f32) = if item.source() == entry.source_text()
    {
        ("source", 1.0)
    } else if normalize_source(item.source()) == normalize_source(entry.source_text()) {
        ("normalized_source", 0.98)
    } else if loose_source(item.source()) == loose_source(entry.source_text()) {
        ("fuzzy_source", 0.75)
    } else {
        return None;
    };

    if context_conflicts(entry, item.context()) {
        confidence = confidence.min(0.90);
    }
    if item.quality() == MemoryQuality::Machine {
        confidence = confidence.min(0.70);
    }

    Some(MemoryCandidate {
        id: item.id().to_string(),
        target: item.target().to_string(),
        layer: item.layer().to_string(),
        confidence,
        match_kind,
        quality: item.quality(),
    })
}

fn memory_annotation(candidate: &MemoryCandidate, processor: &str) -> PipelineAnnotation {
    PipelineAnnotation::new(
        "memory",
        &candidate.id,
        &candidate.layer,
        candidate.confidence,
        candidate.match_kind,
        processor,
        json!({
            "target": candidate.target,
            "quality": memory_quality_name(candidate.quality),
        }),
    )
}

fn context_conflicts(entry: &PipelineEntry, context: &BTreeMap<String, String>) -> bool {
    context
        .iter()
        .any(|(key, value)| entry.entry_value(key).is_some_and(|actual| actual != value))
}

fn normalize_source(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn loose_source(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn rule_annotation(rule: &ReplacementRule, processor: &str) -> PipelineAnnotation {
    PipelineAnnotation::new(
        "replacement_rule",
        rule.id(),
        rule.layer(),
        1.0,
        rule_mode_name(rule.mode()),
        processor,
        annotation_payload(&[
            ("pattern", rule.pattern()),
            ("replacement", rule.replacement()),
        ]),
    )
}

fn rule_mode_name(mode: crate::knowledge::RuleMode) -> &'static str {
    match mode {
        crate::knowledge::RuleMode::Literal => "literal",
        crate::knowledge::RuleMode::Regex => "regex",
    }
}

fn validate_placeholders(entry: &mut PipelineEntry) {
    let Some(translated) = entry.translated_text() else {
        return;
    };
    let source = extract_placeholders(entry.source_text());
    let target = extract_placeholders(translated);
    if source != target {
        entry.add_diagnostic(PipelineDiagnostic::new(
            PipelineDiagnosticSeverity::Warning,
            "placeholder.mismatch",
            "Source and translation placeholders differ.",
            entry.id(),
        ));
    }
}

fn validate_scaleform_newline(entry: &mut PipelineEntry) {
    if entry.kind().as_str() != "scaleform" {
        return;
    }
    if entry
        .translated_text()
        .is_some_and(|text| text.contains(['\r', '\n']))
    {
        entry.add_diagnostic(PipelineDiagnostic::new(
            PipelineDiagnosticSeverity::Warning,
            "scaleform.newline",
            "Scaleform translation contains a newline.",
            entry.id(),
        ));
    }
}

fn validate_empty_translation(entry: &mut PipelineEntry) {
    if entry.translated_text().is_none()
        || entry
            .translated_text()
            .is_some_and(|text| text.trim().is_empty())
    {
        entry.add_diagnostic(PipelineDiagnostic::new(
            PipelineDiagnosticSeverity::Info,
            "translation.empty",
            "Translation is empty.",
            entry.id(),
        ));
    }
}

fn extract_placeholders(text: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '{'
            && let Some(end) = chars[index + 1..].iter().position(|ch| *ch == '}')
        {
            let end = index + 1 + end;
            let value = chars[index..=end].iter().collect::<String>();
            if value.len() > 2 && !value.contains(char::is_whitespace) {
                placeholders.push(value);
            }
            index = end + 1;
            continue;
        }
        if chars[index] == '%'
            && index + 1 < chars.len()
            && matches!(chars[index + 1], 's' | 'd' | 'f' | 'i')
        {
            placeholders.push(chars[index..=index + 1].iter().collect());
            index += 2;
            continue;
        }
        index += 1;
    }
    placeholders.sort();
    placeholders
}

fn term_status_name(status: TermStatus) -> &'static str {
    match status {
        TermStatus::Preferred => "preferred",
        TermStatus::Allowed => "allowed",
        TermStatus::Forbidden => "forbidden",
    }
}

fn memory_quality_name(quality: MemoryQuality) -> &'static str {
    match quality {
        MemoryQuality::Confirmed => "confirmed",
        MemoryQuality::Imported => "imported",
        MemoryQuality::Machine => "machine",
        MemoryQuality::Rejected => "rejected",
    }
}
