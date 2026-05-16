use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use stringer_pipeline::{
    KnowledgeBase, KnowledgeLayer, MemoryQuality, PipelineDiagnostic, PipelineDiagnosticSeverity,
    PipelineEntry, Term, TermInput, TermStatus, TranslationMemoryEntry,
    TranslationMemoryEntryInput,
};
use stringer_workspace_core::{WorkspaceCoreError, WorkspaceSettings};

use crate::KnowledgeError;
use crate::candidates::{
    CandidateStore, DEFAULT_IN_MEMORY_CANDIDATE_BUDGET_BYTES, InMemoryCandidateStore,
    SqliteCandidateStore, estimate_candidate_bytes,
};
use crate::index::{
    EntryCandidateIndexReader, EntryKnowledgeQuery, IndexedEntryKnowledge, IndexedKnowledgeId,
    KnowledgeFileKind, KnowledgeSourceFile, ensure_knowledge_index, read_index_diagnostics,
    read_index_knowledge_ids, read_matching_index_knowledge_ids,
};
use crate::layers::{
    GLOBAL_LAYER, KnowledgeLayerSelection, WORKSPACE_LAYER, collect_knowledge_resources,
};

#[derive(Debug)]
pub(crate) struct LayeredKnowledgeSession {
    indexes: Vec<KnowledgeIndexHandle>,
    suppressed_items: BTreeSet<IndexedKnowledgeId>,
    diagnostics: Vec<PipelineDiagnostic>,
}

#[derive(Debug)]
enum CandidateSource {
    InMemory(Box<InMemoryCandidateStore>),
    Sqlite(SqliteCandidateStore),
}

impl LayeredKnowledgeSession {
    pub(crate) fn open(
        workspace: &Utf8Path,
        settings: &WorkspaceSettings,
    ) -> Result<Self, KnowledgeError> {
        let resources =
            collect_knowledge_resources(workspace, settings, KnowledgeLayerSelection::All)?;
        let mut indexes = Vec::new();
        for layer in &resources.layers {
            let index = ensure_knowledge_index(&layer.index_path, &layer.files, settings, || {
                load_knowledge_from_files(&layer.files)
            })?;
            indexes.push(KnowledgeIndexHandle {
                path: index.path,
                layer: layer.name().to_string(),
            });
        }
        let overrides = cross_layer_overrides(&indexes)?;
        let suppressed_items = suppressed_index_items(&overrides);
        let diagnostics = read_layered_index_diagnostics(&indexes, &overrides)?;
        Ok(Self {
            indexes,
            suppressed_items,
            diagnostics,
        })
    }

    pub(crate) fn index_paths(&self) -> Vec<Utf8PathBuf> {
        self.indexes
            .iter()
            .map(|index| index.path.clone())
            .collect()
    }

    pub(crate) fn suppressed_items(&self) -> &BTreeSet<IndexedKnowledgeId> {
        &self.suppressed_items
    }

    pub(crate) fn diagnostics(&self) -> &[PipelineDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn has_indexes(&self) -> bool {
        !self.indexes.is_empty()
    }

    pub(crate) fn open_candidate_store(&self) -> Result<LayeredCandidateStore, KnowledgeError> {
        let index_paths = self.index_paths();
        let readers = open_candidate_readers(&index_paths)?;
        let estimated_bytes = estimate_candidate_bytes(&readers)?;
        let source = if estimated_bytes <= DEFAULT_IN_MEMORY_CANDIDATE_BUDGET_BYTES {
            match InMemoryCandidateStore::from_index_readers(&readers, &self.suppressed_items) {
                Ok(store) => CandidateSource::InMemory(Box::new(store)),
                Err(KnowledgeError::CandidateIndex { .. }) => CandidateSource::Sqlite(
                    SqliteCandidateStore::open(&index_paths, &self.suppressed_items)?,
                ),
                Err(error) => return Err(error),
            }
        } else {
            CandidateSource::Sqlite(SqliteCandidateStore::open(
                &index_paths,
                &self.suppressed_items,
            )?)
        };
        Ok(LayeredCandidateStore { source })
    }
}

#[derive(Debug)]
pub(crate) struct LayeredCandidateStore {
    source: CandidateSource,
}

impl LayeredCandidateStore {
    pub(crate) fn in_memory_store(&self) -> Option<&InMemoryCandidateStore> {
        match &self.source {
            CandidateSource::InMemory(store) => Some(store.as_ref()),
            CandidateSource::Sqlite(_) => None,
        }
    }
}

impl CandidateStore for LayeredCandidateStore {
    fn candidates_for_query(
        &self,
        query: &EntryKnowledgeQuery,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError> {
        match &self.source {
            CandidateSource::InMemory(store) => store.candidates_for_query(query),
            CandidateSource::Sqlite(store) => store.candidates_for_query(query),
        }
    }
}

pub(crate) fn candidate_knowledge_for_entry(
    store: &impl CandidateStore,
    entry: &PipelineEntry,
) -> Result<KnowledgeBase, KnowledgeError> {
    let query = EntryKnowledgeQuery::from_entry(entry);
    knowledge_from_index_candidates(store.candidates_for_query(&query)?)
}

pub(crate) fn load_knowledge_from_files(
    files: &[KnowledgeSourceFile],
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
    let ordered = ["built-in", GLOBAL_LAYER, WORKSPACE_LAYER]
        .into_iter()
        .filter_map(|name| layers.remove(name))
        .collect::<Vec<_>>();
    KnowledgeBase::from_layers(ordered).map_err(KnowledgeError::from)
}

fn knowledge_from_index_candidates(
    candidates: IndexedEntryKnowledge,
) -> Result<KnowledgeBase, KnowledgeError> {
    let mut global = KnowledgeLayer::new(GLOBAL_LAYER);
    let mut workspace = KnowledgeLayer::new(WORKSPACE_LAYER);
    for term in candidates.terms {
        let target = layer_mut(&mut global, &mut workspace, &term.layer);
        target.push_term(Term::new(TermInput {
            id: term.id,
            source: term.source,
            target: term.target,
            aliases: term.aliases,
            case_sensitive: term.case_sensitive,
            status: TermStatus::from_name(&term.status).unwrap_or_default(),
            scope: term.scope,
            tags: Vec::new(),
            note: None,
            layer: term.layer,
        }));
    }
    for item in candidates.memory {
        let target = layer_mut(&mut global, &mut workspace, &item.layer);
        target.push_memory(TranslationMemoryEntry::new(TranslationMemoryEntryInput {
            id: item.id,
            source: item.source,
            target: item.target,
            source_locale: item.source_locale,
            target_locale: item.target_locale,
            context: item.context,
            quality: MemoryQuality::from_name(&item.quality).unwrap_or_default(),
            layer: item.layer,
        }));
    }
    KnowledgeBase::from_layers(vec![global, workspace]).map_err(KnowledgeError::from)
}

fn open_candidate_readers(
    paths: &[Utf8PathBuf],
) -> Result<Vec<EntryCandidateIndexReader>, KnowledgeError> {
    paths
        .iter()
        .map(|path| EntryCandidateIndexReader::open(path))
        .collect()
}

fn layer_mut<'a>(
    global: &'a mut KnowledgeLayer,
    workspace: &'a mut KnowledgeLayer,
    layer: &str,
) -> &'a mut KnowledgeLayer {
    match layer {
        WORKSPACE_LAYER => workspace,
        _ => global,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KnowledgeIndexHandle {
    path: Utf8PathBuf,
    layer: String,
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
                    .or_insert_with(|| index.layer.clone());
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
