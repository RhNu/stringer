use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use aho_corasick::AhoCorasick;

use crate::KnowledgeError;
use crate::index::{
    EntryCandidateIndexReader, EntryKnowledgeQuery, IndexedEntryKnowledge, IndexedKnowledgeId,
    IndexedMemoryCandidate, IndexedTermCandidate, normalize_lookup_text,
};

pub(crate) const DEFAULT_IN_MEMORY_CANDIDATE_BUDGET_BYTES: usize = 1024 * 1024 * 1024;

pub(crate) trait CandidateStore {
    fn candidates_for_query(
        &self,
        query: &EntryKnowledgeQuery,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError>;
}

#[derive(Debug)]
pub(crate) struct InMemoryCandidateStore {
    terms: Vec<IndexedTermCandidate>,
    memory: Vec<IndexedMemoryCandidate>,
    exact_memory: HashMap<MemoryLookupKey, Vec<usize>>,
    normalized_memory: HashMap<MemoryLookupKey, Vec<usize>>,
    loose_memory: HashMap<MemoryLookupKey, Vec<usize>>,
    case_sensitive_terms: TermMatcher,
    case_insensitive_terms: TermMatcher,
}

impl InMemoryCandidateStore {
    pub(crate) fn from_index_readers(
        readers: &[EntryCandidateIndexReader],
        suppressed_items: &BTreeSet<IndexedKnowledgeId>,
    ) -> Result<Self, KnowledgeError> {
        let mut candidates = IndexedEntryKnowledge::default();
        for reader in readers {
            candidates.extend(reader.read_all_entry_candidates(suppressed_items)?);
        }
        Self::from_indexed(candidates, suppressed_items)
    }

    pub(crate) fn from_indexed(
        mut candidates: IndexedEntryKnowledge,
        suppressed_items: &BTreeSet<IndexedKnowledgeId>,
    ) -> Result<Self, KnowledgeError> {
        candidates.terms.retain(|term| {
            !suppressed_items.contains(&IndexedKnowledgeId {
                kind: "term".to_string(),
                id: term.id.clone(),
                layer: term.layer.clone(),
            })
        });
        candidates.memory.retain(|item| {
            !suppressed_items.contains(&IndexedKnowledgeId {
                kind: "memory".to_string(),
                id: item.id.clone(),
                layer: item.layer.clone(),
            })
        });

        let case_sensitive_terms =
            TermMatcher::build(&candidates.terms, TermMatcherMode::CaseSensitive)?;
        let case_insensitive_terms =
            TermMatcher::build(&candidates.terms, TermMatcherMode::CaseInsensitive)?;
        let mut store = Self {
            terms: candidates.terms,
            memory: candidates.memory,
            exact_memory: HashMap::new(),
            normalized_memory: HashMap::new(),
            loose_memory: HashMap::new(),
            case_sensitive_terms,
            case_insensitive_terms,
        };
        store.index_memory();
        Ok(store)
    }

    fn index_memory(&mut self) {
        for (index, item) in self.memory.iter().enumerate() {
            self.exact_memory
                .entry(MemoryLookupKey::new(
                    item.source_locale.clone(),
                    item.target_locale.clone(),
                    item.source.clone(),
                ))
                .or_default()
                .push(index);
            self.normalized_memory
                .entry(MemoryLookupKey::new(
                    item.source_locale.clone(),
                    item.target_locale.clone(),
                    normalize_lookup_text(&item.source),
                ))
                .or_default()
                .push(index);
            self.loose_memory
                .entry(MemoryLookupKey::new(
                    item.source_locale.clone(),
                    item.target_locale.clone(),
                    crate::index::normalize_loose_text(&item.source),
                ))
                .or_default()
                .push(index);
        }
    }
}

impl CandidateStore for InMemoryCandidateStore {
    fn candidates_for_query(
        &self,
        query: &EntryKnowledgeQuery,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError> {
        let mut term_indexes = BTreeSet::new();
        self.case_sensitive_terms
            .extend_matches(&query.source, &mut term_indexes);
        self.case_insensitive_terms
            .extend_matches(&query.source_norm, &mut term_indexes);

        let mut memory_indexes = BTreeSet::new();
        extend_memory_matches(
            &self.exact_memory,
            query,
            &query.source,
            &mut memory_indexes,
        );
        extend_memory_matches(
            &self.normalized_memory,
            query,
            &query.source_norm,
            &mut memory_indexes,
        );
        extend_memory_matches(
            &self.loose_memory,
            query,
            &query.source_loose,
            &mut memory_indexes,
        );

        Ok(IndexedEntryKnowledge {
            terms: term_indexes
                .into_iter()
                .map(|index| self.terms[index].clone())
                .collect(),
            memory: memory_indexes
                .into_iter()
                .map(|index| self.memory[index].clone())
                .collect(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct SqliteCandidateStore {
    readers: Vec<EntryCandidateIndexReader>,
    suppressed_items: BTreeSet<IndexedKnowledgeId>,
}

impl SqliteCandidateStore {
    pub(crate) fn open(
        paths: &[camino::Utf8PathBuf],
        suppressed_items: &BTreeSet<IndexedKnowledgeId>,
    ) -> Result<Self, KnowledgeError> {
        let mut readers = Vec::with_capacity(paths.len());
        for path in paths {
            readers.push(EntryCandidateIndexReader::open(path)?);
        }
        Ok(Self {
            readers,
            suppressed_items: suppressed_items.clone(),
        })
    }
}

impl CandidateStore for SqliteCandidateStore {
    fn candidates_for_query(
        &self,
        query: &EntryKnowledgeQuery,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError> {
        let mut candidates = IndexedEntryKnowledge::default();
        for reader in &self.readers {
            candidates.extend(reader.read_entry_candidates(query, &self.suppressed_items)?);
        }
        Ok(candidates)
    }
}

pub(crate) fn estimate_candidate_bytes(
    readers: &[EntryCandidateIndexReader],
) -> Result<usize, KnowledgeError> {
    let mut total = 0usize;
    for reader in readers {
        total = total.saturating_add(reader.estimate_entry_candidate_bytes()?);
    }
    Ok(total)
}

#[derive(Debug, Clone)]
struct TermMatcher {
    matcher: Option<AhoCorasick>,
    always_terms: Vec<usize>,
    pattern_terms: Vec<usize>,
}

impl TermMatcher {
    fn build(
        terms: &[IndexedTermCandidate],
        mode: TermMatcherMode,
    ) -> Result<Self, KnowledgeError> {
        let mut patterns = Vec::new();
        let mut always_terms = BTreeSet::new();
        let mut pattern_terms = Vec::new();
        for (index, term) in terms.iter().enumerate() {
            if !mode.matches_term(term.case_sensitive) {
                continue;
            }
            for value in
                std::iter::once(term.source.as_str()).chain(term.aliases.iter().map(String::as_str))
            {
                if value.is_empty() {
                    always_terms.insert(index);
                    continue;
                }
                patterns.push(mode.pattern(value));
                pattern_terms.push(index);
            }
        }
        if patterns.is_empty() {
            return Ok(Self {
                matcher: None,
                always_terms: always_terms.into_iter().collect(),
                pattern_terms,
            });
        }
        let matcher =
            AhoCorasick::new(patterns).map_err(|source| KnowledgeError::CandidateIndex {
                message: source.to_string(),
            })?;
        Ok(Self {
            matcher: Some(matcher),
            always_terms: always_terms.into_iter().collect(),
            pattern_terms,
        })
    }

    fn extend_matches(&self, haystack: &str, indexes: &mut BTreeSet<usize>) {
        indexes.extend(self.always_terms.iter().copied());
        let Some(matcher) = &self.matcher else {
            return;
        };
        for matched in matcher.find_overlapping_iter(haystack) {
            indexes.insert(self.pattern_terms[matched.pattern().as_usize()]);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TermMatcherMode {
    CaseSensitive,
    CaseInsensitive,
}

impl TermMatcherMode {
    fn matches_term(self, case_sensitive: bool) -> bool {
        matches!(
            (self, case_sensitive),
            (Self::CaseSensitive, true) | (Self::CaseInsensitive, false)
        )
    }

    fn pattern(self, value: &str) -> String {
        match self {
            Self::CaseSensitive => value.to_string(),
            Self::CaseInsensitive => normalize_lookup_text(value),
        }
    }
}

#[derive(Debug, Clone, Eq)]
struct MemoryLookupKey {
    source_locale: String,
    target_locale: String,
    source: String,
}

impl MemoryLookupKey {
    fn new(source_locale: String, target_locale: String, source: String) -> Self {
        Self {
            source_locale,
            target_locale,
            source,
        }
    }
}

impl PartialEq for MemoryLookupKey {
    fn eq(&self, other: &Self) -> bool {
        self.source_locale == other.source_locale
            && self.target_locale == other.target_locale
            && self.source == other.source
    }
}

impl Hash for MemoryLookupKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_locale.hash(state);
        self.target_locale.hash(state);
        self.source.hash(state);
    }
}

fn extend_memory_matches(
    index: &HashMap<MemoryLookupKey, Vec<usize>>,
    query: &EntryKnowledgeQuery,
    source: &str,
    matches: &mut BTreeSet<usize>,
) {
    let key = MemoryLookupKey::new(
        query.source_locale.clone(),
        query.target_locale.clone(),
        source.to_string(),
    );
    if let Some(indexes) = index.get(&key) {
        matches.extend(indexes.iter().copied());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::index::{
        EntryKnowledgeQuery, IndexedEntryKnowledge, IndexedKnowledgeId, IndexedMemoryCandidate,
        IndexedTermCandidate,
    };

    use super::{CandidateStore, InMemoryCandidateStore};

    #[test]
    fn in_memory_memory_lookup_uses_all_source_keys_and_deduplicates_rows() {
        let store = InMemoryCandidateStore::from_indexed(
            IndexedEntryKnowledge {
                memory: vec![memory(7, "tm:iron", "Iron Sword")],
                ..IndexedEntryKnowledge::default()
            },
            &BTreeSet::new(),
        )
        .unwrap();

        let exact = store.candidates_for_query(&query("Iron Sword")).unwrap();
        let normalized = store.candidates_for_query(&query("Iron   Sword")).unwrap();
        let loose = store.candidates_for_query(&query("Iron-Sword")).unwrap();

        assert_eq!(exact.memory.len(), 1);
        assert_eq!(normalized.memory.len(), 1);
        assert_eq!(loose.memory.len(), 1);
        assert_eq!(exact.memory[0].id, "tm:iron");
    }

    #[test]
    fn in_memory_terms_match_source_and_alias_without_false_case_sensitive_hit() {
        let store = InMemoryCandidateStore::from_indexed(
            IndexedEntryKnowledge {
                terms: vec![
                    term(1, "term:case", "Dragonborn", Vec::new(), true),
                    term(2, "term:alias", "Dovahkiin", vec!["DB"], false),
                ],
                ..IndexedEntryKnowledge::default()
            },
            &BTreeSet::new(),
        )
        .unwrap();

        let candidates = store
            .candidates_for_query(&query("dragonborn meets db"))
            .unwrap();
        let ids = candidates
            .terms
            .iter()
            .map(|term| term.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["term:alias"]);
    }

    #[test]
    fn in_memory_store_excludes_suppressed_global_items() {
        let suppressed = BTreeSet::from([IndexedKnowledgeId {
            kind: "memory".to_string(),
            id: "tm:shared".to_string(),
            layer: "global".to_string(),
        }]);
        let store = InMemoryCandidateStore::from_indexed(
            IndexedEntryKnowledge {
                memory: vec![
                    memory_in_layer(1, "tm:shared", "global", "Iron Sword", "全局铁剑"),
                    memory_in_layer(2, "tm:shared", "workspace", "Iron Sword", "工作区铁剑"),
                ],
                ..IndexedEntryKnowledge::default()
            },
            &suppressed,
        )
        .unwrap();

        let candidates = store.candidates_for_query(&query("Iron Sword")).unwrap();

        assert_eq!(candidates.memory.len(), 1);
        assert_eq!(candidates.memory[0].layer, "workspace");
        assert_eq!(candidates.memory[0].target, "工作区铁剑");
    }

    #[test]
    fn in_memory_terms_keep_overlapping_matches() {
        let store = InMemoryCandidateStore::from_indexed(
            IndexedEntryKnowledge {
                terms: vec![
                    term(1, "term:iron", "Iron", Vec::new(), false),
                    term(2, "term:iron_sword", "Iron Sword", Vec::new(), false),
                ],
                ..IndexedEntryKnowledge::default()
            },
            &BTreeSet::new(),
        )
        .unwrap();

        let candidates = store.candidates_for_query(&query("Iron Sword")).unwrap();
        let ids = candidates
            .terms
            .iter()
            .map(|term| term.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["term:iron", "term:iron_sword"]);
    }

    #[test]
    fn in_memory_terms_include_empty_literal_candidates() {
        let store = InMemoryCandidateStore::from_indexed(
            IndexedEntryKnowledge {
                terms: vec![
                    term(1, "term:empty_source", "", Vec::new(), false),
                    term(2, "term:empty_alias", "Unmatched", vec![""], true),
                ],
                ..IndexedEntryKnowledge::default()
            },
            &BTreeSet::new(),
        )
        .unwrap();

        let candidates = store.candidates_for_query(&query("Iron Sword")).unwrap();
        let ids = candidates
            .terms
            .iter()
            .map(|term| term.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, vec!["term:empty_source", "term:empty_alias"]);
    }

    fn query(source: &str) -> EntryKnowledgeQuery {
        EntryKnowledgeQuery {
            source: source.to_string(),
            source_norm: crate::index::normalize_lookup_text(source),
            source_loose: crate::index::normalize_loose_text(source),
            source_locale: "en".to_string(),
            target_locale: "zh-Hans".to_string(),
        }
    }

    fn term(
        rowid: i64,
        id: &str,
        source: &str,
        aliases: Vec<&str>,
        case_sensitive: bool,
    ) -> IndexedTermCandidate {
        IndexedTermCandidate {
            rowid,
            id: id.to_string(),
            layer: "workspace".to_string(),
            source: source.to_string(),
            target: format!("{source} target"),
            aliases: aliases.into_iter().map(str::to_string).collect(),
            case_sensitive,
            status: "preferred".to_string(),
            scope: BTreeMap::new(),
        }
    }

    fn memory(rowid: i64, id: &str, source: &str) -> IndexedMemoryCandidate {
        memory_in_layer(rowid, id, "workspace", source, "目标")
    }

    fn memory_in_layer(
        rowid: i64,
        id: &str,
        layer: &str,
        source: &str,
        target: &str,
    ) -> IndexedMemoryCandidate {
        IndexedMemoryCandidate {
            rowid,
            id: id.to_string(),
            layer: layer.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_locale: "en".to_string(),
            target_locale: "zh-Hans".to_string(),
            quality: "confirmed".to_string(),
            context: BTreeMap::new(),
        }
    }
}
