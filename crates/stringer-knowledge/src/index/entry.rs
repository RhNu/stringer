use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::{Connection, params};

use super::IndexedKnowledgeId;
use crate::KnowledgeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntryKnowledgeQuery {
    pub(crate) source: String,
    pub(crate) source_norm: String,
    pub(crate) source_loose: String,
    pub(crate) source_locale: String,
    pub(crate) target_locale: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct IndexedEntryKnowledge {
    pub(crate) terms: Vec<IndexedTermCandidate>,
    pub(crate) memory: Vec<IndexedMemoryCandidate>,
}

impl IndexedEntryKnowledge {
    fn extend(&mut self, other: Self) {
        self.terms.extend(other.terms);
        self.memory.extend(other.memory);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedTermCandidate {
    pub(crate) id: String,
    pub(crate) layer: String,
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) case_sensitive: bool,
    pub(crate) status: String,
    pub(crate) scope: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedMemoryCandidate {
    pub(crate) id: String,
    pub(crate) layer: String,
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) source_locale: String,
    pub(crate) target_locale: String,
    pub(crate) quality: String,
    pub(crate) context: BTreeMap<String, String>,
}

pub(crate) fn read_entry_candidate_knowledge(
    paths: &[Utf8PathBuf],
    query: &EntryKnowledgeQuery,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<IndexedEntryKnowledge, KnowledgeError> {
    let mut candidates = IndexedEntryKnowledge::default();
    for path in paths {
        candidates.extend(read_entry_candidates_for_index(
            path,
            query,
            suppressed_items,
        )?);
    }
    Ok(candidates)
}

fn read_entry_candidates_for_index(
    path: &Utf8Path,
    query: &EntryKnowledgeQuery,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<IndexedEntryKnowledge, KnowledgeError> {
    let connection = Connection::open(path).map_err(|source| KnowledgeError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    Ok(IndexedEntryKnowledge {
        terms: read_entry_term_candidates(&connection, path, query, suppressed_items)?,
        memory: read_entry_memory_candidates(&connection, path, query, suppressed_items)?,
    })
}

fn read_entry_term_candidates(
    connection: &Connection,
    path: &Utf8Path,
    query: &EntryKnowledgeQuery,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<IndexedTermCandidate>, KnowledgeError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT rowid, id, layer, source, target, status, case_sensitive ",
            "FROM items WHERE item_kind = 'term' AND ",
            "(?1 LIKE '%' || source_norm || '%' OR EXISTS (",
            "SELECT 1 FROM aliases WHERE aliases.item_rowid = items.rowid ",
            "AND ?1 LIKE '%' || alias_norm || '%'))"
        ))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![query.source_norm.as_str()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, bool>(6)?,
            ))
        })
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut terms = Vec::new();
    for row in rows {
        let (rowid, id, layer, source, target, status, case_sensitive) =
            row.map_err(|source| KnowledgeError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        if suppressed_items.contains(&IndexedKnowledgeId {
            kind: "term".to_string(),
            id: id.clone(),
            layer: layer.clone(),
        }) {
            continue;
        }
        terms.push(IndexedTermCandidate {
            id,
            layer,
            source,
            target,
            aliases: aliases_for_item(connection, path, rowid)?,
            case_sensitive,
            status: status.unwrap_or_else(|| "preferred".to_string()),
            scope: scope_values_for_item(connection, path, rowid)?,
        });
    }
    Ok(terms)
}

fn read_entry_memory_candidates(
    connection: &Connection,
    path: &Utf8Path,
    query: &EntryKnowledgeQuery,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<IndexedMemoryCandidate>, KnowledgeError> {
    let mut statement = connection
        .prepare(concat!(
            "SELECT rowid, id, layer, source, target, source_locale, target_locale, quality ",
            "FROM items WHERE item_kind = 'memory' AND source_locale = ?1 ",
            "AND target_locale = ?2 AND (source = ?3 OR source_norm = ?4 OR source_loose = ?5)"
        ))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(
            params![
                query.source_locale.as_str(),
                query.target_locale.as_str(),
                query.source.as_str(),
                query.source_norm.as_str(),
                query.source_loose.as_str(),
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut memory = Vec::new();
    for row in rows {
        let (rowid, id, layer, source, target, source_locale, target_locale, quality) = row
            .map_err(|source| KnowledgeError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        if suppressed_items.contains(&IndexedKnowledgeId {
            kind: "memory".to_string(),
            id: id.clone(),
            layer: layer.clone(),
        }) {
            continue;
        }
        memory.push(IndexedMemoryCandidate {
            id,
            layer,
            source,
            target,
            source_locale,
            target_locale,
            quality: quality.unwrap_or_else(|| "imported".to_string()),
            context: memory_context_for_item(connection, path, rowid)?,
        });
    }
    Ok(memory)
}

fn aliases_for_item(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<Vec<String>, KnowledgeError> {
    let mut statement = connection
        .prepare("SELECT alias FROM aliases WHERE item_rowid = ?1")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![rowid], |row| row.get::<_, String>(0))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut aliases = Vec::new();
    for row in rows {
        aliases.push(row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(aliases)
}

fn scope_values_for_item(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<BTreeMap<String, Vec<String>>, KnowledgeError> {
    let mut scope = BTreeMap::<String, Vec<String>>::new();
    for (key, value) in item_scope_rows(connection, path, rowid)? {
        scope.entry(key).or_default().push(value);
    }
    Ok(scope)
}

fn memory_context_for_item(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<BTreeMap<String, String>, KnowledgeError> {
    Ok(item_scope_rows(connection, path, rowid)?
        .into_iter()
        .collect())
}

fn item_scope_rows(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
) -> Result<Vec<(String, String)>, KnowledgeError> {
    let mut statement = connection
        .prepare("SELECT key, value FROM item_scope WHERE item_rowid = ?1")
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map(params![rowid], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(values)
}
