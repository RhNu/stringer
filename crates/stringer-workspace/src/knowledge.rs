use std::collections::BTreeMap;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use stringer_pipeline::{
    BasicValidationProcessor, KnowledgeBase, KnowledgeLayer, Pipeline, PipelineDiagnostic,
    PipelineDiagnosticSeverity, PipelineEntry, PipelineEntryKind, PipelineOptions, PipelineStage,
    ReplacementRuleProcessor, TerminologyProcessor, TranslationMemoryProcessor,
};

use crate::WorkspaceError;
use crate::batch::claimed_entry_ids;
use crate::knowledge_index::{
    KnowledgeFileKind, KnowledgeSourceFile, ensure_knowledge_index, index_is_current,
    knowledge_index_path, read_index_diagnostics, rebuild_knowledge_index, source_file_from_path,
};
use crate::knowledge_lookup::{
    KnowledgeLookup, KnowledgeSearchOptions, LookupKnowledgeField, LookupKnowledgeMode,
    LookupKnowledgeSource, search_knowledge_index,
};
use crate::lock::{WorkspaceLock, unix_ms};
use crate::package::{
    TranslationMeta, TranslationRecord, read_translation_package_records,
    write_translation_package_records,
};
use crate::settings::{WorkspaceSettings, game_release_name, load_global_knowledge_root};

const BUILTIN_PROCESSORS: &[&str] = &[
    "stringer.term",
    "stringer.memory",
    "stringer.validation",
    "stringer.replacement",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotateTranslationsOptions {
    pub project_root: Utf8PathBuf,
    pub workspace: Utf8PathBuf,
    pub skip_memory_fill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateTranslationsOptions {
    pub project_root: Utf8PathBuf,
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupKnowledgeOptions {
    pub project_root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub text: String,
    pub kind: PipelineEntryKind,
    pub context: Vec<(String, String)>,
    pub mode: LookupKnowledgeMode,
    pub source: LookupKnowledgeSource,
    pub field: LookupKnowledgeField,
    pub limit: usize,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadKnowledgeLayersOptions {
    pub project_root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub prefer_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildKnowledgeIndexOptions {
    pub project_root: Utf8PathBuf,
    pub settings: WorkspaceSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeIndexSummary {
    pub files: usize,
    pub terms: usize,
    pub memory: usize,
    pub rules: usize,
    pub diagnostics: usize,
    pub indexed_items: usize,
    pub fts_rows: usize,
    pub rebuild_reason: Option<String>,
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

pub fn annotate_translations(
    options: AnnotateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut package = read_translation_package_records(&options.workspace)?;
    let claimed = claimed_entry_ids(&options.workspace)?;
    let settings = settings_with_user_knowledge_defaults(package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        project_root: options.project_root.clone(),
        settings,
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
            let annotate_report = pipeline.run_stage(
                PipelineStage::Annotate,
                std::slice::from_mut(&mut entry),
                &loaded.knowledge,
                &PipelineOptions::default(),
            );
            let memory_options = PipelineOptions {
                allow_memory_auto_fill: should_fill_memory(
                    record,
                    claimed.contains(&record.id),
                    options.skip_memory_fill,
                ),
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
            if memory_report.auto_filled > 0 {
                record.translation_meta = Some(TranslationMeta {
                    origin: Some("memory".to_string()),
                    updated_at_unix_ms: Some(unix_ms()),
                });
            }
            summary.entries += 1;
            summary.annotations += annotate_report.annotations + memory_report.annotations;
            summary.diagnostics += diagnostics;
            summary.auto_filled += memory_report.auto_filled;
        }
    }

    write_translation_package_records(&options.workspace, &package)?;
    Ok(summary)
}

pub fn validate_translations(
    options: ValidateTranslationsOptions,
) -> Result<KnowledgeSummary, WorkspaceError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut package = read_translation_package_records(&options.workspace)?;
    let settings = settings_with_user_knowledge_defaults(package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        project_root: options.project_root.clone(),
        settings,
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

    write_translation_package_records(&options.workspace, &package)?;
    Ok(summary)
}

pub fn lookup_knowledge(
    options: LookupKnowledgeOptions,
) -> Result<KnowledgeLookup, WorkspaceError> {
    let query = options.text.clone();
    let settings = options.settings.clone();
    let mut context = BTreeMap::new();
    context.insert(
        "game".to_string(),
        game_release_name(settings.game_release).to_string(),
    );
    context.insert("kind".to_string(), options.kind.as_str().to_string());
    context.insert("source_locale".to_string(), settings.source_locale.clone());
    context.insert("target_locale".to_string(), settings.target_locale.clone());
    for (key, value) in options.context {
        context.insert(key, value);
    }
    let files = collect_source_files(&options.project_root, &settings)?;
    let index_path = knowledge_index_path(&options.project_root);
    let index = ensure_knowledge_index(&index_path, &files, &settings, || {
        load_knowledge_from_files(&files)
    })?;
    let search = search_knowledge_index(
        &index.path,
        &KnowledgeSearchOptions {
            query: &query,
            mode: options.mode,
            source: options.source,
            field: options.field,
            limit: options.limit,
            case_sensitive: options.case_sensitive,
            source_locale: &settings.source_locale,
            target_locale: &settings.target_locale,
            context: &context,
        },
    )?;
    let diagnostics = read_index_diagnostics(&index.path)?;
    Ok(KnowledgeLookup {
        query,
        mode: options.mode,
        total_matches: search.total_matches,
        results: search.results,
        diagnostics,
        index_used: true,
    })
}

pub fn build_knowledge_index(
    options: BuildKnowledgeIndexOptions,
) -> Result<KnowledgeIndexSummary, WorkspaceError> {
    let settings = options.settings;
    let files = collect_source_files(&options.project_root, &settings)?;
    let knowledge = load_knowledge_from_files(&files)?;
    let index_path = knowledge_index_path(&options.project_root);
    rebuild_knowledge_index(&index_path, &files, &settings, &knowledge, Some("explicit"))
}

pub fn load_knowledge_layers(
    options: LoadKnowledgeLayersOptions,
) -> Result<LoadedKnowledgeLayers, WorkspaceError> {
    let settings = options.settings;
    let files = collect_source_files(&options.project_root, &settings)?;
    let index_path = knowledge_index_path(&options.project_root);
    let knowledge = load_knowledge_from_files(&files)?;
    let mut diagnostics = Vec::new();
    let index_used =
        options.prefer_index && index_is_current(&index_path, &files, &settings).unwrap_or(false);
    if options.prefer_index && !index_used {
        diagnostics.push(index_stale_diagnostic());
    }
    Ok(LoadedKnowledgeLayers {
        knowledge,
        diagnostics,
        index_used,
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

fn index_stale_diagnostic() -> PipelineDiagnostic {
    PipelineDiagnostic::new(
        PipelineDiagnosticSeverity::Warning,
        "knowledge.index_stale",
        "Knowledge index is missing or stale; using file-backed knowledge.",
        "",
    )
    .with_layer("index")
    .with_rule_id("knowledge.sqlite")
}

fn settings_with_user_knowledge_defaults(
    mut settings: WorkspaceSettings,
) -> Result<WorkspaceSettings, WorkspaceError> {
    if settings.global_knowledge_root.is_none() {
        settings.global_knowledge_root = load_global_knowledge_root(None)?;
    }
    Ok(settings)
}

fn collect_source_files(
    project_root: &Utf8Path,
    settings: &WorkspaceSettings,
) -> Result<Vec<KnowledgeSourceFile>, WorkspaceError> {
    let mut files = Vec::new();
    for (layer, root) in knowledge_roots(project_root, settings) {
        collect_files_for_layer(&mut files, &layer, &root)?;
    }
    Ok(files)
}

fn knowledge_roots(
    project_root: &Utf8Path,
    settings: &WorkspaceSettings,
) -> Vec<(String, Utf8PathBuf)> {
    let mut roots = Vec::new();
    let project_knowledge_root = project_root.join("knowledge");
    if let Some(global_root) = settings.global_knowledge_root.clone() {
        if !same_path(&global_root, &project_knowledge_root) {
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
    roots.push(("project".to_string(), project_knowledge_root));
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
            files.push(source_file_from_path(path, layer, kind)?);
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
    let ordered = ["built-in", "global", "library", "project"]
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
    collect_sorted_files(root, extension, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_sorted_files(
    root: &Utf8Path,
    extension: &str,
    files: &mut Vec<Utf8PathBuf>,
) -> Result<(), WorkspaceError> {
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
        if path.is_dir() {
            collect_sorted_files(&path, extension, files)?;
        } else if path.extension() == Some(extension) {
            files.push(path);
        }
    }
    Ok(())
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
        record.source.clone(),
        settings.source_locale.clone(),
        settings.target_locale.clone(),
        asset_path.to_string(),
    );
    if let Some(translation) = &record.translation {
        entry.set_translated_text(translation.clone());
    }
    entry.insert_context("game", game_release_name(settings.game_release));
    for (key, value) in &record.context {
        entry.insert_context(key.clone(), value.clone());
    }
    entry.set_annotations(record.hints.clone());
    entry.set_diagnostics(record.diagnostics.clone());
    Ok(entry)
}

fn write_entry_result(record: &mut TranslationRecord, entry: PipelineEntry) {
    let (translation, hints, diagnostics) = entry.into_annotations_and_diagnostics();
    record.translation = translation;
    record.hints = hints;
    record.diagnostics = diagnostics;
}

fn should_fill_memory(record: &TranslationRecord, claimed: bool, skip_memory_fill: bool) -> bool {
    if skip_memory_fill || claimed {
        return false;
    }
    if matches!(
        record
            .translation_meta
            .as_ref()
            .and_then(|meta| meta.origin.as_deref()),
        Some("agent" | "manual")
    ) {
        return false;
    }
    true
}
