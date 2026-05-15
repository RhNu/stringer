use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::Utf8PathBuf;
use stringer_pipeline::{
    BasicValidationProcessor, KnowledgeBase, KnowledgeLayer, Pipeline, PipelineDiagnostic,
    PipelineDiagnosticSeverity, PipelineEntry, PipelineEntryKind, PipelineOptions, PipelineStage,
    ReplacementRuleProcessor, TerminologyProcessor, TranslationMemoryProcessor,
};

use crate::KnowledgeError;
use crate::index::{
    IndexedKnowledgeId, KnowledgeFileKind, ensure_knowledge_index, index_is_current,
    read_index_diagnostics, read_index_knowledge_ids, read_matching_index_knowledge_ids,
    rebuild_knowledge_index,
};
use crate::layers::{
    collect_all_source_files, collect_index_layers, collect_workspace_index_layer,
};
use crate::lookup::{
    KnowledgeLookup, KnowledgeSearchOptions, LookupKnowledgeField, LookupKnowledgeMode,
    LookupKnowledgeSource, search_knowledge_indexes,
};
use stringer_workspace_core::claimed_entry_ids;
use stringer_workspace_core::{
    TranslationMeta, TranslationRecord, read_translation_package_records,
    write_translation_package_records,
};
use stringer_workspace_core::{WorkspaceCoreError, WorkspaceLock, unix_ms};
use stringer_workspace_core::{WorkspaceSettings, game_release_name, load_global_knowledge_root};

const BUILTIN_PROCESSORS: &[&str] = &[
    "stringer.term",
    "stringer.memory",
    "stringer.validation",
    "stringer.replacement",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotateTranslationsOptions {
    pub workspace: Utf8PathBuf,
    pub skip_memory_fill: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateTranslationsOptions {
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupKnowledgeOptions {
    pub workspace: Utf8PathBuf,
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
    pub workspace: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub prefer_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildKnowledgeIndexOptions {
    pub workspace: Utf8PathBuf,
    pub settings: WorkspaceSettings,
    pub scope: KnowledgeIndexBuildScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeIndexBuildScope {
    All,
    Workspace,
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

impl KnowledgeIndexSummary {
    fn add(&mut self, other: Self) {
        self.files += other.files;
        self.terms += other.terms;
        self.memory += other.memory;
        self.rules += other.rules;
        self.diagnostics += other.diagnostics;
        self.indexed_items += other.indexed_items;
        self.fts_rows += other.fts_rows;
        self.rebuild_reason = self.rebuild_reason.take().or(other.rebuild_reason);
    }
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
) -> Result<KnowledgeSummary, KnowledgeError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut package = read_translation_package_records(&options.workspace)?;
    let claimed = claimed_entry_ids(&options.workspace)?;
    let settings = settings_with_user_knowledge_defaults(package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: options.workspace.clone(),
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
) -> Result<KnowledgeSummary, KnowledgeError> {
    let _lock = WorkspaceLock::acquire(&options.workspace)?;
    let mut package = read_translation_package_records(&options.workspace)?;
    let settings = settings_with_user_knowledge_defaults(package.settings.clone())?;
    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: options.workspace.clone(),
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
) -> Result<KnowledgeLookup, KnowledgeError> {
    let query = options.text.clone();
    let settings = settings_with_user_knowledge_defaults(options.settings.clone())?;
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
    let layers = collect_index_layers(&options.workspace, &settings)?;
    let mut indexes = Vec::new();
    for layer in &layers {
        let index = ensure_knowledge_index(&layer.index_path, &layer.files, &settings, || {
            load_knowledge_from_files(&layer.files)
        })?;
        indexes.push(KnowledgeIndexHandle { path: index.path });
    }
    let overrides = cross_layer_overrides(&indexes)?;
    let suppressed_items = suppressed_index_items(&overrides);
    let index_paths = indexes
        .iter()
        .map(|index| index.path.clone())
        .collect::<Vec<_>>();
    let search = search_knowledge_indexes(
        &index_paths,
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
        &suppressed_items,
    )?;
    let diagnostics = read_layered_index_diagnostics(&indexes, &overrides)?;
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
) -> Result<KnowledgeIndexSummary, KnowledgeError> {
    let settings = settings_for_build_scope(options.settings, options.scope)?;
    let layers = match options.scope {
        KnowledgeIndexBuildScope::All => collect_index_layers(&options.workspace, &settings)?,
        KnowledgeIndexBuildScope::Workspace => {
            vec![collect_workspace_index_layer(&options.workspace)?]
        }
    };
    let mut summary = KnowledgeIndexSummary::default();
    for layer in &layers {
        let knowledge = load_knowledge_from_files(&layer.files)?;
        summary.add(rebuild_knowledge_index(
            &layer.index_path,
            &layer.files,
            &settings,
            &knowledge,
            Some("explicit"),
        )?);
    }
    Ok(summary)
}

pub fn load_knowledge_layers(
    options: LoadKnowledgeLayersOptions,
) -> Result<LoadedKnowledgeLayers, KnowledgeError> {
    let settings = settings_with_user_knowledge_defaults(options.settings)?;
    let layers = collect_index_layers(&options.workspace, &settings)?;
    let files = collect_all_source_files(&layers);
    let knowledge = load_knowledge_from_files(&files)?;
    let mut diagnostics = Vec::new();
    let index_used = options.prefer_index
        && layers.iter().all(|layer| {
            index_is_current(&layer.index_path, &layer.files, &settings).unwrap_or(false)
        });
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct KnowledgeIndexHandle {
    path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KnowledgeOverride {
    kind: String,
    id: String,
    old_layer: String,
    new_layer: String,
}

fn read_layered_index_diagnostics(
    indexes: &[KnowledgeIndexHandle],
    overrides: &[KnowledgeOverride],
) -> Result<Vec<PipelineDiagnostic>, KnowledgeError> {
    let suppressed_rules = suppressed_rule_diagnostics(overrides);
    let mut diagnostics = Vec::new();
    for index in indexes {
        diagnostics.extend(
            read_index_diagnostics(&index.path)?
                .into_iter()
                .filter(|diagnostic| !diagnostic_is_suppressed(diagnostic, &suppressed_rules)),
        );
    }
    diagnostics.extend(cross_layer_override_diagnostics(overrides));
    Ok(diagnostics)
}

fn cross_layer_overrides(
    indexes: &[KnowledgeIndexHandle],
) -> Result<Vec<KnowledgeOverride>, KnowledgeError> {
    if indexes.len() < 2 {
        return Ok(Vec::new());
    }

    let mut higher_layers = BTreeMap::<(String, String), String>::new();
    let mut overrides = BTreeMap::<(String, String, String, String), KnowledgeOverride>::new();
    for (position, index) in indexes.iter().enumerate().rev() {
        if !higher_layers.is_empty() {
            let keys = higher_layers.keys().cloned().collect::<BTreeSet<_>>();
            for id in read_matching_index_knowledge_ids(&index.path, &keys)? {
                let key = (id.kind.clone(), id.id.clone());
                if let Some(new_layer) = higher_layers.get(&key)
                    && new_layer != &id.layer
                {
                    overrides.insert(
                        (
                            id.kind.clone(),
                            id.id.clone(),
                            id.layer.clone(),
                            new_layer.clone(),
                        ),
                        KnowledgeOverride {
                            kind: id.kind,
                            id: id.id,
                            old_layer: id.layer,
                            new_layer: new_layer.clone(),
                        },
                    );
                }
            }
        }

        if position > 0 {
            for id in read_index_knowledge_ids(&index.path)? {
                higher_layers
                    .entry((id.kind, id.id))
                    .or_insert_with(|| id.layer);
            }
        }
    }

    Ok(overrides.into_values().collect())
}

fn suppressed_index_items(overrides: &[KnowledgeOverride]) -> BTreeSet<IndexedKnowledgeId> {
    overrides
        .iter()
        .map(|item| IndexedKnowledgeId {
            kind: item.kind.clone(),
            id: item.id.clone(),
            layer: item.old_layer.clone(),
        })
        .collect()
}

fn suppressed_rule_diagnostics(overrides: &[KnowledgeOverride]) -> BTreeSet<IndexedKnowledgeId> {
    overrides
        .iter()
        .filter(|item| item.kind == "rule")
        .map(|item| IndexedKnowledgeId {
            kind: item.kind.clone(),
            id: item.id.clone(),
            layer: item.old_layer.clone(),
        })
        .collect()
}

fn diagnostic_is_suppressed(
    diagnostic: &PipelineDiagnostic,
    suppressed_rules: &BTreeSet<IndexedKnowledgeId>,
) -> bool {
    let Some(layer) = diagnostic.layer() else {
        return false;
    };
    let Some(rule_id) = diagnostic.rule_id() else {
        return false;
    };
    suppressed_rules.contains(&IndexedKnowledgeId {
        kind: "rule".to_string(),
        id: rule_id.to_string(),
        layer: layer.to_string(),
    })
}

fn cross_layer_override_diagnostics(overrides: &[KnowledgeOverride]) -> Vec<PipelineDiagnostic> {
    overrides
        .iter()
        .map(|item| override_diagnostic(&item.kind, &item.id, &item.old_layer, &item.new_layer))
        .collect()
}

fn override_diagnostic(
    kind: &str,
    id: &str,
    old_layer: &str,
    new_layer: &str,
) -> PipelineDiagnostic {
    let item = match kind {
        "rule" => "replacement rule",
        _ => kind,
    };
    PipelineDiagnostic::new(
        PipelineDiagnosticSeverity::Warning,
        "knowledge.override",
        format!("{new_layer} {item} `{id}` overrides {old_layer} {item} `{id}`"),
        "",
    )
    .with_layer(new_layer)
    .with_rule_id(id)
}

fn settings_with_user_knowledge_defaults(
    mut settings: WorkspaceSettings,
) -> Result<WorkspaceSettings, KnowledgeError> {
    if settings.global_knowledge_root.is_none() {
        settings.global_knowledge_root = load_global_knowledge_root(None)?;
    }
    Ok(settings)
}

fn settings_for_build_scope(
    settings: WorkspaceSettings,
    scope: KnowledgeIndexBuildScope,
) -> Result<WorkspaceSettings, KnowledgeError> {
    settings_for_build_scope_with(settings, scope, || Ok(load_global_knowledge_root(None)?))
}

fn settings_for_build_scope_with(
    mut settings: WorkspaceSettings,
    scope: KnowledgeIndexBuildScope,
    load_global_root: impl FnOnce() -> Result<Option<Utf8PathBuf>, KnowledgeError>,
) -> Result<WorkspaceSettings, KnowledgeError> {
    if scope == KnowledgeIndexBuildScope::All && settings.global_knowledge_root.is_none() {
        settings.global_knowledge_root = load_global_root()?;
    }
    Ok(settings)
}

fn load_knowledge_from_files(
    files: &[crate::index::KnowledgeSourceFile],
) -> Result<KnowledgeBase, KnowledgeError> {
    let mut layers = BTreeMap::<String, KnowledgeLayer>::new();
    layers.insert("built-in".to_string(), KnowledgeLayer::new("built-in"));
    for file in files {
        let layer = layers
            .entry(file.layer.clone())
            .or_insert_with(|| KnowledgeLayer::new(&file.layer));
        let text =
            fs::read_to_string(&file.path).map_err(|source| WorkspaceCoreError::ReadFile {
                path: file.path.clone(),
                source,
            })?;
        match file.kind {
            KnowledgeFileKind::Terms => layer.add_terms_toml(file.path.as_str(), &text)?,
            KnowledgeFileKind::Memory => layer.add_memory_jsonl(file.path.as_str(), &text)?,
            KnowledgeFileKind::Rules => layer.add_rules_toml(file.path.as_str(), &text)?,
        }
    }
    let ordered = ["built-in", "global", "workspace"]
        .into_iter()
        .filter_map(|name| layers.remove(name))
        .collect::<Vec<_>>();
    KnowledgeBase::from_layers(ordered).map_err(KnowledgeError::from)
}

fn entry_from_record(
    record: &TranslationRecord,
    kind: &str,
    asset_path: &str,
    settings: &WorkspaceSettings,
) -> Result<PipelineEntry, KnowledgeError> {
    let kind = PipelineEntryKind::from_package_kind(kind).ok_or_else(|| {
        WorkspaceCoreError::InvalidTranslationPackagePath {
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

#[cfg(test)]
mod tests {
    use super::*;
    use stringer_core::Language;
    use stringer_plugin::GameRelease;

    #[test]
    fn workspace_scope_build_settings_do_not_resolve_global_defaults() {
        let settings = settings_for_build_scope_with(
            test_settings(),
            KnowledgeIndexBuildScope::Workspace,
            || panic!("workspace scoped index rebuild should not read global defaults"),
        )
        .unwrap();

        assert_eq!(settings.global_knowledge_root, None);
    }

    #[test]
    fn all_scope_build_settings_resolve_global_defaults() {
        let settings =
            settings_for_build_scope_with(test_settings(), KnowledgeIndexBuildScope::All, || {
                Ok(Some(Utf8PathBuf::from("global-knowledge")))
            })
            .unwrap();

        assert_eq!(
            settings.global_knowledge_root,
            Some(Utf8PathBuf::from("global-knowledge"))
        );
    }

    fn test_settings() -> WorkspaceSettings {
        WorkspaceSettings {
            game_release: GameRelease::SkyrimSe,
            asset_language: Language::English,
            source_locale: "en".to_string(),
            target_locale: "zh-Hans".to_string(),
            global_knowledge_root: None,
        }
    }
}
