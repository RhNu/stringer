use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use stringer_pipeline::{
    BasicValidationProcessor, KnowledgeBase, KnowledgeLayer, Pipeline, PipelineAnnotation,
    PipelineDiagnostic, PipelineEntry, PipelineEntryKind, PipelineOptions, PipelineStage,
    ReplacementRuleProcessor, TerminologyProcessor, TranslationMemoryProcessor,
};

use crate::WorkspaceError;
use crate::package::{
    TranslationRecord, read_translation_package_records, write_translation_package_records,
};
use crate::settings::{WorkspaceSettings, game_release_name};

const BUILTIN_PROCESSORS: &[&str] = &[
    "stringer.term",
    "stringer.memory",
    "stringer.validation",
    "stringer.replacement",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotateTranslationsOptions {
    pub root: Utf8PathBuf,
    pub translations: Utf8PathBuf,
    pub allow_memory_auto_fill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateTranslationsOptions {
    pub root: Utf8PathBuf,
    pub translations: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupKnowledgeOptions {
    pub root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub text: String,
    pub kind: PipelineEntryKind,
    pub context: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeSummary {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: usize,
    pub auto_filled: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeLookup {
    pub annotations: Vec<PipelineAnnotation>,
    pub diagnostics: Vec<PipelineDiagnostic>,
}

pub fn annotate_translations(
    options: AnnotateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let knowledge = load_knowledge_layers(&options.root)?;
    let mut package = read_translation_package_records(&options.translations)?;
    let pipeline = default_pipeline();
    let mut summary = KnowledgeSummary::default();

    for file in &mut package.files {
        for record in &mut file.records {
            let mut entry = entry_from_record(
                record,
                &file.manifest_file.kind,
                &file.manifest_file.asset_path,
                &package.settings,
            )?;
            entry.clear_annotations_from_processors(BUILTIN_PROCESSORS);
            entry.clear_diagnostics();
            let annotate_report = pipeline.run_stage(
                PipelineStage::Annotate,
                std::slice::from_mut(&mut entry),
                &knowledge,
                &PipelineOptions::default(),
            );
            let memory_options = PipelineOptions {
                allow_memory_auto_fill: options.allow_memory_auto_fill,
                ..PipelineOptions::default()
            };
            let memory_report = pipeline.run_stage(
                PipelineStage::MemoryApply,
                std::slice::from_mut(&mut entry),
                &knowledge,
                &memory_options,
            );
            write_entry_result(record, entry);
            summary.entries += 1;
            summary.annotations += annotate_report.annotations + memory_report.annotations;
            summary.diagnostics +=
                annotate_report.diagnostics.len() + memory_report.diagnostics.len();
            summary.auto_filled += memory_report.auto_filled;
        }
    }

    write_translation_package_records(&options.translations, &package)?;
    Ok(summary)
}

pub fn validate_translations(
    options: ValidateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let knowledge = load_knowledge_layers(&options.root)?;
    let mut package = read_translation_package_records(&options.translations)?;
    let pipeline = default_pipeline();
    let mut summary = KnowledgeSummary::default();

    for file in &mut package.files {
        for record in &mut file.records {
            let mut entry = entry_from_record(
                record,
                &file.manifest_file.kind,
                &file.manifest_file.asset_path,
                &package.settings,
            )?;
            entry.clear_diagnostics();
            let report = pipeline.run_stage(
                PipelineStage::Validate,
                std::slice::from_mut(&mut entry),
                &knowledge,
                &PipelineOptions::default(),
            );
            write_entry_result(record, entry);
            summary.entries += 1;
            summary.annotations += report.annotations;
            summary.diagnostics += report.diagnostics.len();
        }
    }

    write_translation_package_records(&options.translations, &package)?;
    Ok(summary)
}

pub fn lookup_knowledge(
    options: LookupKnowledgeOptions,
) -> Result<KnowledgeLookup, WorkspaceError> {
    let knowledge = load_knowledge_layers(&options.root)?;
    let pipeline = default_pipeline();
    let mut entry = PipelineEntry::new(
        "lookup",
        options.kind,
        options.text,
        options.settings.source_locale,
        options.settings.target_locale,
        "",
    );
    entry.insert_context("game", game_release_name(options.settings.game_release));
    for (key, value) in options.context {
        entry.insert_context(key, value);
    }
    pipeline.run_stage(
        PipelineStage::Annotate,
        std::slice::from_mut(&mut entry),
        &knowledge,
        &PipelineOptions::default(),
    );
    Ok(KnowledgeLookup {
        annotations: entry.annotations().to_vec(),
        diagnostics: entry.diagnostics().to_vec(),
    })
}

pub fn load_knowledge_layers(root: &Utf8Path) -> Result<KnowledgeBase, WorkspaceError> {
    let mut project = KnowledgeLayer::new("project");
    load_terms(root, &mut project)?;
    load_memory(root, &mut project)?;
    load_rules(root, &mut project)?;
    KnowledgeBase::from_layers(vec![project]).map_err(WorkspaceError::from)
}

fn default_pipeline() -> Pipeline {
    Pipeline::new(vec![
        Box::new(TerminologyProcessor),
        Box::new(TranslationMemoryProcessor),
        Box::new(BasicValidationProcessor),
        Box::new(ReplacementRuleProcessor),
    ])
}

fn load_terms(root: &Utf8Path, layer: &mut KnowledgeLayer) -> Result<(), WorkspaceError> {
    for path in sorted_files(&root.join("knowledge/terms"), "toml")? {
        let text = fs::read_to_string(&path).map_err(|source| WorkspaceError::ReadFile {
            path: path.clone(),
            source,
        })?;
        layer.add_terms_toml(path.as_str(), &text)?;
    }
    Ok(())
}

fn load_memory(root: &Utf8Path, layer: &mut KnowledgeLayer) -> Result<(), WorkspaceError> {
    for path in sorted_files(&root.join("knowledge/memory"), "jsonl")? {
        let text = fs::read_to_string(&path).map_err(|source| WorkspaceError::ReadFile {
            path: path.clone(),
            source,
        })?;
        layer.add_memory_jsonl(path.as_str(), &text)?;
    }
    Ok(())
}

fn load_rules(root: &Utf8Path, layer: &mut KnowledgeLayer) -> Result<(), WorkspaceError> {
    for path in sorted_files(&root.join("knowledge/rules"), "toml")? {
        let text = fs::read_to_string(&path).map_err(|source| WorkspaceError::ReadFile {
            path: path.clone(),
            source,
        })?;
        layer.add_rules_toml(path.as_str(), &text)?;
    }
    Ok(())
}

fn sorted_files(root: &Utf8Path, extension: &str) -> Result<Vec<Utf8PathBuf>, WorkspaceError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(root).map_err(|source| WorkspaceError::ReadFile {
        path: root.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| WorkspaceError::ReadFile {
            path: root.to_owned(),
            source,
        })?;
        let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|path| {
            WorkspaceError::InvalidLogicalPath {
                path: path.display().to_string(),
                message: "knowledge file path is not valid UTF-8".to_string(),
            }
        })?;
        if path.extension() == Some(extension) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn entry_from_record(
    record: &TranslationRecord,
    kind: &str,
    asset_path: &str,
    settings: &WorkspaceSettings,
) -> Result<PipelineEntry, WorkspaceError> {
    let kind = PipelineEntryKind::from_package_kind(kind).ok_or_else(|| {
        WorkspaceError::InvalidTranslationPackagePath {
            path: kind.to_string(),
            message: "unsupported translation package kind".to_string(),
        }
    })?;
    let mut entry = PipelineEntry::new(
        record.id.clone(),
        kind,
        record.source_text.clone(),
        settings.source_locale.clone(),
        settings.target_locale.clone(),
        asset_path.to_string(),
    );
    if let Some(translated_text) = &record.translated_text {
        entry.set_translated_text(translated_text.clone());
    }
    entry.insert_context("game", game_release_name(settings.game_release));
    for (key, value) in &record.context {
        entry.insert_context(key.clone(), value.clone());
    }
    merge_source_context(&mut entry, &record.source);
    entry.set_annotations(record.annotations.clone());
    entry.set_diagnostics(record.diagnostics.clone());
    Ok(entry)
}

fn merge_source_context(entry: &mut PipelineEntry, source: &Option<Value>) {
    let Some(Value::Object(values)) = source else {
        return;
    };
    for (key, value) in values {
        if entry.context().contains_key(key) {
            continue;
        }
        match value {
            Value::String(text) => {
                entry.insert_context(key.clone(), text.clone());
            }
            Value::Number(number) => {
                entry.insert_context(key.clone(), number.to_string());
            }
            Value::Bool(flag) => {
                entry.insert_context(key.clone(), flag.to_string());
            }
            _ => {}
        }
    }
}

fn write_entry_result(record: &mut TranslationRecord, entry: PipelineEntry) {
    let (translated_text, annotations, diagnostics) = entry.into_annotations_and_diagnostics();
    record.translated_text = translated_text;
    record.annotations = annotations;
    record.diagnostics = diagnostics;
}
