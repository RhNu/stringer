use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::{Connection, OptionalExtension, params};
use stringer_pipeline::{
    KnowledgeBase, MemoryQuality, PipelineDiagnostic, PipelineDiagnosticSeverity,
};

use crate::WorkspaceError;
use crate::knowledge::KnowledgeIndexSummary;
use crate::settings::{WorkspaceSettings, game_release_name};

pub(crate) const KNOWLEDGE_INDEX_SCHEMA_VERSION: &str = "2";
const INDEX_COMPLETE_KEY: &str = "index_complete";

type IndexedFileManifest = BTreeMap<String, (String, String, u64, Option<i64>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnowledgeFileKind {
    Terms,
    Memory,
    Rules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeSourceFile {
    pub(crate) path: Utf8PathBuf,
    pub(crate) layer: String,
    pub(crate) kind: KnowledgeFileKind,
    pub(crate) size: u64,
    pub(crate) modified_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnowledgeIndexState {
    pub(crate) path: Utf8PathBuf,
}

pub(crate) fn knowledge_index_path(root: &Utf8Path) -> Utf8PathBuf {
    root.join(".stringer/indexes/knowledge.sqlite")
}

pub(crate) fn source_file_from_path(
    path: Utf8PathBuf,
    layer: &str,
    kind: KnowledgeFileKind,
) -> Result<KnowledgeSourceFile, WorkspaceError> {
    let metadata = fs::metadata(&path).map_err(|source| WorkspaceError::ReadFile {
        path: path.clone(),
        source,
    })?;
    Ok(KnowledgeSourceFile {
        path,
        layer: layer.to_string(),
        kind,
        size: metadata.len(),
        modified_unix_ms: metadata.modified().ok().and_then(modified_unix_ms),
    })
}

pub(crate) fn ensure_knowledge_index(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
    settings: &WorkspaceSettings,
    knowledge: impl FnOnce() -> Result<KnowledgeBase, WorkspaceError>,
) -> Result<KnowledgeIndexState, WorkspaceError> {
    if index_is_current(path, files, settings).unwrap_or(false) {
        return Ok(KnowledgeIndexState {
            path: path.to_owned(),
        });
    }
    rebuild_knowledge_index(path, files, settings, &knowledge()?, Some("auto"))?;
    Ok(KnowledgeIndexState {
        path: path.to_owned(),
    })
}

pub(crate) fn rebuild_knowledge_index(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
    settings: &WorkspaceSettings,
    knowledge: &KnowledgeBase,
    rebuild_reason: Option<&str>,
) -> Result<KnowledgeIndexSummary, WorkspaceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let temp_path = temporary_index_path(path);
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|source| WorkspaceError::WriteFile {
            path: temp_path.clone(),
            source,
        })?;
    }
    let fts_rows = match build_replacement_index(&temp_path, files, settings, knowledge) {
        Ok(fts_rows) => fts_rows,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    if let Err(error) = replace_index_file(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    Ok(KnowledgeIndexSummary {
        files: files.len(),
        terms: knowledge.terms().len(),
        memory: knowledge.memory().len(),
        rules: knowledge.rules().len(),
        diagnostics: knowledge.merge_diagnostics().len(),
        indexed_items: knowledge.terms().len() + knowledge.memory().len(),
        fts_rows,
        rebuild_reason: rebuild_reason.map(str::to_string),
    })
}

fn build_replacement_index(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
    settings: &WorkspaceSettings,
    knowledge: &KnowledgeBase,
) -> Result<usize, WorkspaceError> {
    let mut connection = Connection::open(path).map_err(|source| WorkspaceError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    let transaction = connection
        .transaction()
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    create_schema(&transaction, path)?;
    write_source_files(&transaction, path, files)?;
    write_items(&transaction, path, knowledge)?;
    write_diagnostics(&transaction, path, knowledge)?;
    write_meta(&transaction, settings, path)?;
    let fts_rows = fts_row_count(&transaction, path)?;
    transaction
        .commit()
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    Ok(fts_rows)
}

pub(crate) fn index_is_current(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
    settings: &WorkspaceSettings,
) -> Result<bool, WorkspaceError> {
    if !path.exists() {
        return Ok(false);
    }
    let connection = Connection::open(path).map_err(|source| WorkspaceError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    if read_meta_value(&connection, path, "schema_version")?.as_deref()
        != Some(KNOWLEDGE_INDEX_SCHEMA_VERSION)
    {
        return Ok(false);
    }
    if read_meta_value(&connection, path, "settings_fingerprint")?.as_deref()
        != Some(settings_fingerprint(settings).as_str())
    {
        return Ok(false);
    }
    if read_meta_value(&connection, path, INDEX_COMPLETE_KEY)?.as_deref() != Some("1") {
        return Ok(false);
    }
    // Freshness intentionally uses file metadata here. A content hash pass on
    // every lookup would make current-index checks linear in total knowledge
    // bytes, which defeats the lookup performance goal.
    Ok(read_indexed_files(&connection, path)? == current_files(files))
}

pub(crate) fn read_index_diagnostics(
    path: &Utf8Path,
) -> Result<Vec<PipelineDiagnostic>, WorkspaceError> {
    let connection = Connection::open(path).map_err(|source| WorkspaceError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    let mut statement = connection
        .prepare("SELECT severity, code, message, entry_id, layer, rule_id FROM diagnostics")
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            let mut diagnostic = PipelineDiagnostic::new(
                severity_from_name(&row.get::<_, String>(0)?),
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            );
            if let Some(layer) = row.get::<_, Option<String>>(4)? {
                diagnostic = diagnostic.with_layer(layer);
            }
            if let Some(rule_id) = row.get::<_, Option<String>>(5)? {
                diagnostic = diagnostic.with_rule_id(rule_id);
            }
            Ok(diagnostic)
        })
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut diagnostics = Vec::new();
    for row in rows {
        diagnostics.push(row.map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?);
    }
    Ok(diagnostics)
}

pub(crate) fn normalize_lookup_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(crate) fn file_kind_name(kind: KnowledgeFileKind) -> &'static str {
    match kind {
        KnowledgeFileKind::Terms => "terms",
        KnowledgeFileKind::Memory => "memory",
        KnowledgeFileKind::Rules => "rules",
    }
}

fn create_schema(connection: &Connection, path: &Utf8Path) -> Result<(), WorkspaceError> {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = OFF;
            CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE source_files(
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                layer TEXT NOT NULL,
                kind TEXT NOT NULL,
                size INTEGER NOT NULL,
                modified_unix_ms INTEGER,
                fingerprint TEXT NOT NULL
            );
            CREATE TABLE items(
                rowid INTEGER PRIMARY KEY,
                item_kind TEXT NOT NULL,
                id TEXT NOT NULL,
                layer TEXT NOT NULL,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                source_norm TEXT NOT NULL,
                target_norm TEXT NOT NULL,
                alias_norm TEXT NOT NULL,
                source_locale TEXT,
                target_locale TEXT,
                quality TEXT,
                status TEXT,
                case_sensitive INTEGER NOT NULL DEFAULT 0,
                source_len INTEGER NOT NULL,
                file_id INTEGER NOT NULL
            );
            CREATE TABLE aliases(
                item_rowid INTEGER NOT NULL,
                alias TEXT NOT NULL,
                alias_norm TEXT NOT NULL
            );
            CREATE TABLE item_scope(
                item_rowid INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            );
            CREATE TABLE diagnostics(
                severity TEXT NOT NULL,
                code TEXT NOT NULL,
                message TEXT NOT NULL,
                entry_id TEXT NOT NULL,
                layer TEXT,
                rule_id TEXT
            );
            CREATE INDEX idx_items_kind_locale ON items(item_kind, source_locale, target_locale);
            CREATE INDEX idx_items_source_norm ON items(source_norm);
            CREATE INDEX idx_items_target_norm ON items(target_norm);
            CREATE INDEX idx_aliases_alias_norm ON aliases(alias_norm);
            CREATE INDEX idx_scope_item_key ON item_scope(item_rowid, key);
            CREATE VIRTUAL TABLE items_fts USING fts5(
                source_norm,
                target_norm,
                alias_norm,
                content='items',
                content_rowid='rowid',
                tokenize='trigram'
            );
            ",
        )
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })
}

fn write_meta(
    connection: &Connection,
    settings: &WorkspaceSettings,
    path: &Utf8Path,
) -> Result<(), WorkspaceError> {
    for (key, value) in [
        (
            "schema_version".to_string(),
            KNOWLEDGE_INDEX_SCHEMA_VERSION.to_string(),
        ),
        (
            "settings_fingerprint".to_string(),
            settings_fingerprint(settings),
        ),
        (INDEX_COMPLETE_KEY.to_string(), "1".to_string()),
    ] {
        connection
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn write_source_files(
    connection: &Connection,
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
) -> Result<(), WorkspaceError> {
    for file in files {
        connection
            .execute(
                concat!(
                    "INSERT INTO source_files(path, layer, kind, size, modified_unix_ms, fingerprint) ",
                    "VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
                ),
                params![
                    file.path.as_str(),
                    file.layer.as_str(),
                    file_kind_name(file.kind),
                    file.size as i64,
                    file.modified_unix_ms,
                    fingerprint(&fs::read(&file.path).map_err(|source| WorkspaceError::ReadFile {
                        path: file.path.clone(),
                        source,
                    })?),
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn write_items(
    connection: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for term in knowledge.terms() {
        let rowid = insert_item(
            connection,
            path,
            IndexedItemInput {
                item_kind: "term",
                id: term.id(),
                layer: term.layer(),
                source: term.source(),
                target: term.target(),
                alias_norm: &term
                    .aliases()
                    .iter()
                    .map(|alias| normalize_lookup_text(alias))
                    .collect::<Vec<_>>()
                    .join(" "),
                source_locale: None,
                target_locale: None,
                quality: None,
                status: Some(term.status().as_str()),
                case_sensitive: term.case_sensitive(),
                source_len: term.source().chars().count(),
                file_id: 0,
            },
        )?;
        for alias in term.aliases() {
            connection
                .execute(
                    "INSERT INTO aliases(item_rowid, alias, alias_norm) VALUES (?1, ?2, ?3)",
                    params![rowid, alias, normalize_lookup_text(alias)],
                )
                .map_err(|source| WorkspaceError::Sqlite {
                    path: path.to_owned(),
                    source,
                })?;
        }
        for (key, values) in term.scope_values() {
            for value in values {
                insert_scope(connection, path, rowid, key, value)?;
            }
        }
    }
    for item in knowledge.memory() {
        if item.quality() == MemoryQuality::Rejected {
            continue;
        }
        let rowid = insert_item(
            connection,
            path,
            IndexedItemInput {
                item_kind: "memory",
                id: item.id(),
                layer: item.layer(),
                source: item.source(),
                target: item.target(),
                alias_norm: "",
                source_locale: Some(item.source_locale()),
                target_locale: Some(item.target_locale()),
                quality: Some(item.quality().as_str()),
                status: None,
                case_sensitive: false,
                source_len: item.source().chars().count(),
                file_id: 0,
            },
        )?;
        for (key, value) in item.context() {
            insert_scope(connection, path, rowid, key, value)?;
        }
    }
    Ok(())
}

struct IndexedItemInput<'a> {
    item_kind: &'a str,
    id: &'a str,
    layer: &'a str,
    source: &'a str,
    target: &'a str,
    alias_norm: &'a str,
    source_locale: Option<&'a str>,
    target_locale: Option<&'a str>,
    quality: Option<&'a str>,
    status: Option<&'a str>,
    case_sensitive: bool,
    source_len: usize,
    file_id: i64,
}

fn insert_item(
    connection: &Connection,
    path: &Utf8Path,
    item: IndexedItemInput<'_>,
) -> Result<i64, WorkspaceError> {
    connection
        .execute(
            concat!(
                "INSERT INTO items(item_kind, id, layer, source, target, source_norm, target_norm, ",
                "alias_norm, source_locale, target_locale, quality, status, case_sensitive, source_len, file_id) ",
                "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
            ),
            params![
                item.item_kind,
                item.id,
                item.layer,
                item.source,
                item.target,
                normalize_lookup_text(item.source),
                normalize_lookup_text(item.target),
                item.alias_norm,
                item.source_locale,
                item.target_locale,
                item.quality,
                item.status,
                item.case_sensitive,
                item.source_len as i64,
                item.file_id,
            ],
        )
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rowid = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO items_fts(rowid, source_norm, target_norm, alias_norm) SELECT rowid, source_norm, target_norm, alias_norm FROM items WHERE rowid = ?1",
            params![rowid],
        )
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    Ok(rowid)
}

fn insert_scope(
    connection: &Connection,
    path: &Utf8Path,
    rowid: i64,
    key: &str,
    value: &str,
) -> Result<(), WorkspaceError> {
    connection
        .execute(
            "INSERT INTO item_scope(item_rowid, key, value) VALUES (?1, ?2, ?3)",
            params![rowid, key, value],
        )
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    Ok(())
}

fn write_diagnostics(
    connection: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for diagnostic in knowledge.merge_diagnostics() {
        connection
            .execute(
                concat!(
                    "INSERT INTO diagnostics(severity, code, message, entry_id, layer, rule_id) ",
                    "VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
                ),
                params![
                    diagnostic.severity().as_str(),
                    diagnostic.code(),
                    diagnostic.message(),
                    diagnostic.entry_id(),
                    diagnostic.layer(),
                    diagnostic.rule_id(),
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn read_meta_value(
    connection: &Connection,
    path: &Utf8Path,
    key: &str,
) -> Result<Option<String>, WorkspaceError> {
    connection
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })
}

fn read_indexed_files(
    connection: &Connection,
    path: &Utf8Path,
) -> Result<IndexedFileManifest, WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT path, layer, kind, size, modified_unix_ms FROM source_files")
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)? as u64,
                    row.get::<_, Option<i64>>(4)?,
                ),
            ))
        })
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let mut files = BTreeMap::new();
    for row in rows {
        let (path, metadata) = row.map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
        files.insert(path, metadata);
    }
    Ok(files)
}

fn current_files(files: &[KnowledgeSourceFile]) -> IndexedFileManifest {
    files
        .iter()
        .map(|file| {
            (
                file.path.as_str().to_string(),
                (
                    file.layer.clone(),
                    file_kind_name(file.kind).to_string(),
                    file.size,
                    file.modified_unix_ms,
                ),
            )
        })
        .collect()
}

fn fts_row_count(connection: &Connection, path: &Utf8Path) -> Result<usize, WorkspaceError> {
    connection
        .query_row("SELECT count(*) FROM items_fts", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|count| count as usize)
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })
}

fn temporary_index_path(path: &Utf8Path) -> Utf8PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_file_name(format!(
        "{}.rebuild-{}-{timestamp}",
        path.file_name().unwrap_or("knowledge.sqlite"),
        std::process::id()
    ))
}

fn replace_index_file(temp_path: &Utf8Path, path: &Utf8Path) -> Result<(), WorkspaceError> {
    replace_file(temp_path.as_std_path(), path.as_std_path()).map_err(|source| {
        WorkspaceError::WriteFile {
            path: path.to_owned(),
            source,
        }
    })
}

fn replace_file(temp_path: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
    match fs::rename(temp_path, path) {
        Ok(()) => Ok(()),
        Err(source)
            if path.exists()
                && matches!(
                    source.kind(),
                    std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
                ) =>
        {
            fs::remove_file(path)?;
            fs::rename(temp_path, path)
        }
        Err(source) => Err(source),
    }
}

fn settings_fingerprint(settings: &WorkspaceSettings) -> String {
    format!(
        "game={};asset={};source={};target={};global={}",
        game_release_name(settings.game_release),
        settings.asset_language.full_name(),
        settings.source_locale,
        settings.target_locale,
        settings
            .global_knowledge_root
            .as_ref()
            .map(|path| path.as_str())
            .unwrap_or("")
    )
}

fn fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn modified_unix_ms(modified: std::time::SystemTime) -> Option<i64> {
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as i64)
}

fn severity_from_name(value: &str) -> PipelineDiagnosticSeverity {
    match value {
        "error" => PipelineDiagnosticSeverity::Error,
        "info" => PipelineDiagnosticSeverity::Info,
        _ => PipelineDiagnosticSeverity::Warning,
    }
}
