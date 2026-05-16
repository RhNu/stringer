use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::{Connection, params};
use stringer_pipeline::PipelineEntry;

use super::IndexedKnowledgeId;
use crate::KnowledgeError;
use crate::index::{normalize_lookup_text, normalize_loose_text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntryKnowledgeQuery {
    pub(crate) source: String,
    pub(crate) source_norm: String,
    pub(crate) source_loose: String,
    pub(crate) source_locale: String,
    pub(crate) target_locale: String,
}

impl EntryKnowledgeQuery {
    pub(crate) fn from_entry(entry: &PipelineEntry) -> Self {
        Self {
            source: entry.source_text().to_string(),
            source_norm: normalize_lookup_text(entry.source_text()),
            source_loose: normalize_loose_text(entry.source_text()),
            source_locale: entry.source_locale().to_string(),
            target_locale: entry.target_locale().to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct IndexedEntryKnowledge {
    pub(crate) terms: Vec<IndexedTermCandidate>,
    pub(crate) memory: Vec<IndexedMemoryCandidate>,
}

impl IndexedEntryKnowledge {
    pub(crate) fn extend(&mut self, other: Self) {
        self.terms.extend(other.terms);
        self.memory.extend(other.memory);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexedTermCandidate {
    pub(crate) rowid: i64,
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
    pub(crate) rowid: i64,
    pub(crate) id: String,
    pub(crate) layer: String,
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) source_locale: String,
    pub(crate) target_locale: String,
    pub(crate) quality: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(crate) struct EntryCandidateIndexReader {
    path: Utf8PathBuf,
    connection: Connection,
}

impl EntryCandidateIndexReader {
    pub(crate) fn open(path: &Utf8Path) -> Result<Self, KnowledgeError> {
        let connection = Connection::open(path).map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self {
            path: path.to_owned(),
            connection,
        })
    }

    pub(crate) fn read_entry_candidates(
        &self,
        query: &EntryKnowledgeQuery,
        suppressed_items: &BTreeSet<IndexedKnowledgeId>,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError> {
        Ok(IndexedEntryKnowledge {
            terms: read_entry_term_candidates(
                &self.connection,
                &self.path,
                query,
                suppressed_items,
            )?,
            memory: read_entry_memory_candidates(
                &self.connection,
                &self.path,
                query,
                suppressed_items,
            )?,
        })
    }

    pub(crate) fn read_all_entry_candidates(
        &self,
        suppressed_items: &BTreeSet<IndexedKnowledgeId>,
    ) -> Result<IndexedEntryKnowledge, KnowledgeError> {
        Ok(IndexedEntryKnowledge {
            terms: read_all_term_candidates(&self.connection, &self.path, suppressed_items)?,
            memory: read_all_memory_candidates(&self.connection, &self.path, suppressed_items)?,
        })
    }

    pub(crate) fn estimate_entry_candidate_bytes(&self) -> Result<usize, KnowledgeError> {
        let item_bytes = self
            .connection
            .query_row(
                concat!(
                    "SELECT COALESCE(SUM(160 + length(id) + length(layer) + length(source) + ",
                    "length(target) + COALESCE(length(source_locale), 0) + ",
                    "COALESCE(length(target_locale), 0) + COALESCE(length(quality), 0) + ",
                    "COALESCE(length(status), 0)), 0) FROM items ",
                    "WHERE item_kind IN ('term', 'memory')"
                ),
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|source| KnowledgeError::Sqlite {
                path: self.path.clone(),
                source,
            })?;
        let alias_bytes = self
            .connection
            .query_row(
                "SELECT COALESCE(SUM(48 + length(alias) + length(alias_norm)), 0) FROM aliases",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|source| KnowledgeError::Sqlite {
                path: self.path.clone(),
                source,
            })?;
        let scope_bytes = self
            .connection
            .query_row(
                "SELECT COALESCE(SUM(48 + length(key) + length(value)), 0) FROM item_scope",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|source| KnowledgeError::Sqlite {
                path: self.path.clone(),
                source,
            })?;
        Ok((item_bytes as usize)
            .saturating_add(alias_bytes as usize)
            .saturating_add(scope_bytes as usize))
    }
}

fn read_entry_term_candidates(
    connection: &Connection,
    path: &Utf8Path,
    query: &EntryKnowledgeQuery,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<IndexedTermCandidate>, KnowledgeError> {
    let mut statement = connection
        .prepare_cached(concat!(
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
            rowid,
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
        .prepare_cached(memory_candidate_sql())
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
            rowid,
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

pub(crate) fn memory_candidate_sql() -> &'static str {
    concat!(
        "SELECT rowid, id, layer, source, target, source_locale, target_locale, quality ",
        "FROM items WHERE item_kind = 'memory' AND source_locale = ?1 ",
        "AND target_locale = ?2 AND source = ?3 ",
        "UNION ",
        "SELECT rowid, id, layer, source, target, source_locale, target_locale, quality ",
        "FROM items WHERE item_kind = 'memory' AND source_locale = ?1 ",
        "AND target_locale = ?2 AND source_norm = ?4 ",
        "UNION ",
        "SELECT rowid, id, layer, source, target, source_locale, target_locale, quality ",
        "FROM items WHERE item_kind = 'memory' AND source_locale = ?1 ",
        "AND target_locale = ?2 AND source_loose = ?5"
    )
}

fn read_all_term_candidates(
    connection: &Connection,
    path: &Utf8Path,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<IndexedTermCandidate>, KnowledgeError> {
    let mut statement = connection
        .prepare_cached(concat!(
            "SELECT rowid, id, layer, source, target, status, case_sensitive ",
            "FROM items WHERE item_kind = 'term'"
        ))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
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
            rowid,
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

fn read_all_memory_candidates(
    connection: &Connection,
    path: &Utf8Path,
    suppressed_items: &BTreeSet<IndexedKnowledgeId>,
) -> Result<Vec<IndexedMemoryCandidate>, KnowledgeError> {
    let mut statement = connection
        .prepare_cached(concat!(
            "SELECT rowid, id, layer, source, target, source_locale, target_locale, quality ",
            "FROM items WHERE item_kind = 'memory'"
        ))
        .map_err(|source| KnowledgeError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
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
        })
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
            rowid,
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
        .prepare_cached("SELECT alias FROM aliases WHERE item_rowid = ?1")
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
        .prepare_cached("SELECT key, value FROM item_scope WHERE item_rowid = ?1")
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::index::{IndexedItemInput, create_indexes, create_schema, insert_item};

    #[test]
    fn sqlite_memory_candidate_query_uses_composite_source_indexes() {
        let connection = Connection::open_in_memory().unwrap();
        create_schema(&connection, camino::Utf8Path::new("memory-index.sqlite")).unwrap();
        insert_item(
            &connection,
            camino::Utf8Path::new("memory-index.sqlite"),
            IndexedItemInput {
                item_kind: "memory",
                id: "tm:iron",
                layer: "workspace",
                source: "Iron Sword",
                target: "铁剑",
                alias_norm: "",
                source_locale: Some("en"),
                target_locale: Some("zh-Hans"),
                quality: Some("confirmed"),
                status: None,
                case_sensitive: false,
                source_len: "Iron Sword".chars().count(),
                file_id: 0,
            },
        )
        .unwrap();
        create_indexes(&connection, camino::Utf8Path::new("memory-index.sqlite")).unwrap();

        let plan = connection
            .prepare(memory_candidate_sql())
            .unwrap()
            .query_map(
                rusqlite::params!["en", "zh-Hans", "Iron Sword", "iron sword", "ironsword"],
                |_| Ok(()),
            )
            .unwrap()
            .count();
        assert_eq!(plan, 1);

        let explain = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {}", memory_candidate_sql()))
            .unwrap()
            .query_map(
                rusqlite::params!["en", "zh-Hans", "Iron Sword", "iron sword", "ironsword"],
                |row| row.get::<_, String>(3),
            )
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");

        assert!(explain.contains("idx_items_memory_source_exact"));
        assert!(explain.contains("idx_items_memory_source_norm"));
        assert!(explain.contains("idx_items_memory_source_loose"));
    }
}
