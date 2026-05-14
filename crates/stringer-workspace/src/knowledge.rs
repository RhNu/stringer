use std::collections::BTreeMap;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use stringer_pipeline::{
    BasicValidationProcessor, KnowledgeBase, KnowledgeLayer, Pipeline, PipelineAnnotation,
    PipelineDiagnostic, PipelineDiagnosticSeverity, PipelineEntry, PipelineEntryKind,
    PipelineOptions, PipelineStage, ReplacementRuleProcessor, TerminologyProcessor,
    TranslationMemoryProcessor,
};

use crate::WorkspaceError;
use crate::knowledge_index::{
    KnowledgeFileKind, KnowledgeSourceFile, fingerprint, index_is_fresh, knowledge_index_path,
    read_knowledge_index, write_knowledge_index,
};
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
    pub knowledge: KnowledgeLayerOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateTranslationsOptions {
    pub root: Utf8PathBuf,
    pub translations: Utf8PathBuf,
    pub knowledge: KnowledgeLayerOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupKnowledgeOptions {
    pub root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub text: String,
    pub kind: PipelineEntryKind,
    pub context: Vec<(String, String)>,
    pub knowledge: KnowledgeLayerOverrides,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeLayerOverrides {
    pub global_root: Option<Utf8PathBuf>,
    pub override_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadKnowledgeLayersOptions {
    pub root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub knowledge: KnowledgeLayerOverrides,
    pub prefer_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildKnowledgeIndexOptions {
    pub root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub knowledge: KnowledgeLayerOverrides,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeIndexSummary {
    pub files: usize,
    pub terms: usize,
    pub memory: usize,
    pub rules: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedKnowledgeLayers {
    pub knowledge: KnowledgeBase,
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub index_used: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeSummary {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: usize,
    pub auto_filled: usize,
    pub knowledge_diagnostics: Vec<PipelineDiagnostic>,
    pub index_used: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KnowledgeLookup {
    pub annotations: Vec<PipelineAnnotation>,
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub index_used: bool,
}

pub fn annotate_translations(
    options: AnnotateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let mut package = read_translation_package_records(&options.translations)?;
    let settings =
        settings_with_project_knowledge_defaults(&options.root, package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        root: options.root.clone(),
        settings,
        knowledge: options.knowledge,
        prefer_index: true,
    })?;
    let pipeline = default_pipeline();
    let mut summary = KnowledgeSummary {
        knowledge_diagnostics: knowledge_diagnostics(&loaded),
        index_used: loaded.index_used,
        ..KnowledgeSummary::default()
    };

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
                &loaded.knowledge,
                &PipelineOptions::default(),
            );
            let memory_options = PipelineOptions {
                allow_memory_auto_fill: options.allow_memory_auto_fill,
                ..PipelineOptions::default()
            };
            let memory_report = pipeline.run_stage(
                PipelineStage::MemoryApply,
                std::slice::from_mut(&mut entry),
                &loaded.knowledge,
                &memory_options,
            );
            let diagnostics = entry.diagnostics().len();
            write_entry_result(record, entry);
            summary.entries += 1;
            summary.annotations += annotate_report.annotations + memory_report.annotations;
            summary.diagnostics += diagnostics;
            summary.auto_filled += memory_report.auto_filled;
        }
    }

    write_translation_package_records(&options.translations, &package)?;
    Ok(summary)
}

pub fn validate_translations(
    options: ValidateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let mut package = read_translation_package_records(&options.translations)?;
    let settings =
        settings_with_project_knowledge_defaults(&options.root, package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        root: options.root.clone(),
        settings,
        knowledge: options.knowledge,
        prefer_index: true,
    })?;
    let pipeline = default_pipeline();
    let mut summary = KnowledgeSummary {
        knowledge_diagnostics: knowledge_diagnostics(&loaded),
        index_used: loaded.index_used,
        ..KnowledgeSummary::default()
    };

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
                &loaded.knowledge,
                &PipelineOptions::default(),
            );
            let diagnostics = entry.diagnostics().len();
            write_entry_result(record, entry);
            summary.entries += 1;
            summary.annotations += report.annotations;
            summary.diagnostics += diagnostics;
        }
    }

    write_translation_package_records(&options.translations, &package)?;
    Ok(summary)
}

pub fn lookup_knowledge(
    options: LookupKnowledgeOptions,
) -> Result<KnowledgeLookup, WorkspaceError> {
    let settings =
        settings_with_project_knowledge_defaults(&options.root, options.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        root: options.root.clone(),
        settings,
        knowledge: options.knowledge,
        prefer_index: true,
    })?;
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
    let report = pipeline.run_stage(
        PipelineStage::Annotate,
        std::slice::from_mut(&mut entry),
        &loaded.knowledge,
        &PipelineOptions::default(),
    );
    let mut diagnostics = loaded.diagnostics;
    diagnostics.extend(report.diagnostics);
    Ok(KnowledgeLookup {
        annotations: entry.annotations().to_vec(),
        diagnostics,
        index_used: loaded.index_used,
    })
}

pub fn build_knowledge_index(
    options: BuildKnowledgeIndexOptions,
) -> Result<KnowledgeIndexSummary, WorkspaceError> {
    let settings = settings_with_project_knowledge_defaults(&options.root, options.settings)?;
    let files = collect_source_files(&options.root, &settings, &options.knowledge)?;
    let knowledge = load_knowledge_from_files(&files)?;
    let index_path = knowledge_index_path(&options.root);
    write_knowledge_index(&index_path, &files, &knowledge)?;
    Ok(KnowledgeIndexSummary {
        files: files.len(),
        terms: knowledge.terms().len(),
        memory: knowledge.memory().len(),
        rules: knowledge.rules().len(),
        diagnostics: knowledge.merge_diagnostics().len(),
    })
}

pub fn load_knowledge_layers(
    options: LoadKnowledgeLayersOptions,
) -> Result<LoadedKnowledgeLayers, WorkspaceError> {
    let settings = settings_with_project_knowledge_defaults(&options.root, options.settings)?;
    let files = collect_source_files(&options.root, &settings, &options.knowledge)?;
    let index_path = knowledge_index_path(&options.root);
    if options.prefer_index
        && index_path.exists()
        && let Ok(true) = index_is_fresh(&index_path, &files)
        && let Ok(knowledge) = read_knowledge_index(&index_path)
    {
        return Ok(LoadedKnowledgeLayers {
            knowledge,
            diagnostics: Vec::new(),
            index_used: true,
        });
    }

    let knowledge = load_knowledge_from_files(&files)?;
    let mut diagnostics = Vec::new();
    if options.prefer_index {
        let diagnostic = PipelineDiagnostic::new(
            PipelineDiagnosticSeverity::Warning,
            "knowledge.index_stale",
            "Knowledge index is missing or stale; using file-backed knowledge.",
            "",
        )
        .with_layer("index")
        .with_rule_id("knowledge.sqlite");
        diagnostics.push(diagnostic);
    }
    Ok(LoadedKnowledgeLayers {
        knowledge,
        diagnostics,
        index_used: false,
    })
}

fn default_pipeline() -> Pipeline {
    Pipeline::new(vec![
        Box::new(TerminologyProcessor),
        Box::new(TranslationMemoryProcessor),
        Box::new(BasicValidationProcessor),
        Box::new(ReplacementRuleProcessor),
    ])
}

fn knowledge_diagnostics(loaded: &LoadedKnowledgeLayers) -> Vec<PipelineDiagnostic> {
    let mut diagnostics = loaded.diagnostics.clone();
    diagnostics.extend(loaded.knowledge.merge_diagnostics().iter().cloned());
    diagnostics
}

fn settings_with_project_knowledge_defaults(
    root: &Utf8Path,
    mut settings: WorkspaceSettings,
) -> Result<WorkspaceSettings, WorkspaceError> {
    if settings.global_knowledge_root.is_none() {
        settings.global_knowledge_root = project_config_global_root(root)?;
    }
    Ok(settings)
}

fn project_config_global_root(root: &Utf8Path) -> Result<Option<Utf8PathBuf>, WorkspaceError> {
    let config_path = root.join("stringer.toml");
    if !config_path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&config_path).map_err(|source| WorkspaceError::ReadFile {
        path: config_path.clone(),
        source,
    })?;
    let value: toml::Value =
        toml::from_str(&text).map_err(|source| WorkspaceError::ConfigToml {
            path: config_path.clone(),
            source,
        })?;
    let configured = value
        .get("knowledge")
        .and_then(|knowledge| knowledge.get("global_root"))
        .and_then(toml::Value::as_str)
        .map(Utf8PathBuf::from);
    Ok(Some(match configured {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => root.join("knowledge"),
    }))
}

fn collect_source_files(
    root: &Utf8Path,
    settings: &WorkspaceSettings,
    overrides: &KnowledgeLayerOverrides,
) -> Result<Vec<KnowledgeSourceFile>, WorkspaceError> {
    let mut files = Vec::new();
    for (layer, root) in knowledge_roots(root, settings, overrides) {
        collect_files_for_layer(&mut files, &layer, &root)?;
    }
    Ok(files)
}

fn knowledge_roots(
    root: &Utf8Path,
    settings: &WorkspaceSettings,
    overrides: &KnowledgeLayerOverrides,
) -> Vec<(String, Utf8PathBuf)> {
    let mut roots = Vec::new();
    let project_root = root.join("knowledge");
    let global_root = overrides
        .global_root
        .clone()
        .or_else(|| settings.global_knowledge_root.clone());
    if let Some(global_root) = global_root {
        if !same_path(&global_root, &project_root) {
            roots.push(("global".to_string(), global_root.clone()));
        }
        roots.push((
            "library".to_string(),
            global_root
                .join("libraries")
                .join(game_release_name(settings.game_release))
                .join(&settings.target_locale),
        ));
    }
    roots.push(("project".to_string(), project_root));
    if let Some(override_root) = &overrides.override_root {
        roots.push(("override".to_string(), override_root.clone()));
    }
    roots
}

fn same_path(left: &Utf8Path, right: &Utf8Path) -> bool {
    left.as_str().replace('\\', "/").to_lowercase()
        == right.as_str().replace('\\', "/").to_lowercase()
}

fn collect_files_for_layer(
    files: &mut Vec<KnowledgeSourceFile>,
    layer: &str,
    root: &Utf8Path,
) -> Result<(), WorkspaceError> {
    for (kind, folder, extension) in [
        (KnowledgeFileKind::Terms, "terms", "toml"),
        (KnowledgeFileKind::Memory, "memory", "jsonl"),
        (KnowledgeFileKind::Rules, "rules", "toml"),
    ] {
        for path in sorted_files(&root.join(folder), extension)? {
            let bytes = fs::read(&path).map_err(|source| WorkspaceError::ReadFile {
                path: path.clone(),
                source,
            })?;
            files.push(KnowledgeSourceFile {
                path,
                layer: layer.to_string(),
                kind,
                fingerprint: fingerprint(&bytes),
            });
        }
    }
    Ok(())
}

fn load_knowledge_from_files(
    files: &[KnowledgeSourceFile],
) -> Result<KnowledgeBase, WorkspaceError> {
    let mut layers = BTreeMap::<String, KnowledgeLayer>::new();
    layers.insert("built-in".to_string(), KnowledgeLayer::new("built-in"));
    for file in files {
        let layer = layers
            .entry(file.layer.clone())
            .or_insert_with(|| KnowledgeLayer::new(&file.layer));
        let text = fs::read_to_string(&file.path).map_err(|source| WorkspaceError::ReadFile {
            path: file.path.clone(),
            source,
        })?;
        match file.kind {
            KnowledgeFileKind::Terms => layer.add_terms_toml(file.path.as_str(), &text)?,
            KnowledgeFileKind::Memory => layer.add_memory_jsonl(file.path.as_str(), &text)?,
            KnowledgeFileKind::Rules => layer.add_rules_toml(file.path.as_str(), &text)?,
        }
    }
    let ordered = ["built-in", "global", "library", "project", "override"]
        .into_iter()
        .filter_map(|name| layers.remove(name))
        .collect::<Vec<_>>();
    KnowledgeBase::from_layers(ordered).map_err(WorkspaceError::from)
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
