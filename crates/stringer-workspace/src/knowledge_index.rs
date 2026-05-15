use std::collections::BTreeMap;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use stringer_pipeline::{
    KnowledgeBase, KnowledgeLayer, PipelineDiagnostic, PipelineDiagnosticSeverity,
};

use crate::WorkspaceError;

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
    pub(crate) fingerprint: String,
}

pub(crate) fn knowledge_index_path(root: &Utf8Path) -> Utf8PathBuf {
    root.join(".stringer/indexes/knowledge.sqlite")
}

pub(crate) fn fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn write_knowledge_index(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
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
    create_index_schema(&transaction, path)?;
    for file in files {
        transaction
            .execute(
                "INSERT INTO knowledge_files(path, layer, kind, fingerprint) VALUES (?1, ?2, ?3, ?4)",
                params![
                    file.path.as_str(),
                    file.layer.as_str(),
                    file_kind_name(file.kind),
                    file.fingerprint.as_str()
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    write_terms(&transaction, path, knowledge)?;
    write_memory(&transaction, path, knowledge)?;
    write_rules(&transaction, path, knowledge)?;
    write_diagnostics(&transaction, path, knowledge)?;
    transaction
        .commit()
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })
}

pub(crate) fn index_is_fresh(
    path: &Utf8Path,
    files: &[KnowledgeSourceFile],
) -> Result<bool, WorkspaceError> {
    let connection = Connection::open(path).map_err(|source| WorkspaceError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    let indexed = read_indexed_files(&connection, path)?;
    let current = files
        .iter()
        .map(|file| {
            (
                file.path.as_str().to_string(),
                (
                    file.layer.clone(),
                    file_kind_name(file.kind).to_string(),
                    file.fingerprint.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    Ok(indexed == current)
}

pub(crate) fn read_knowledge_index(path: &Utf8Path) -> Result<KnowledgeBase, WorkspaceError> {
    let connection = Connection::open(path).map_err(|source| WorkspaceError::Sqlite {
        path: path.to_owned(),
        source,
    })?;
    let mut layers = BTreeMap::<String, LayerData>::new();
    read_index_terms(&connection, path, &mut layers)?;
    read_index_memory(&connection, path, &mut layers)?;
    read_index_rules(&connection, path, &mut layers)?;
    let mut knowledge_layers = Vec::new();
    for layer_name in ["built-in", "global", "library", "project"] {
        let mut layer = KnowledgeLayer::new(layer_name);
        if let Some(data) = layers.remove(layer_name) {
            if !data.terms.is_empty() {
                layer.add_terms_toml(
                    "knowledge.sqlite",
                    &toml_text(&TermsOut { terms: data.terms }, path)?,
                )?;
            }
            if !data.memory.is_empty() {
                for item in data.memory {
                    let mut line = json_string(&item, path)?;
                    line.push('\n');
                    layer.add_memory_jsonl("knowledge.sqlite", &line)?;
                }
            }
            if !data.rules.is_empty() {
                layer.add_rules_toml(
                    "knowledge.sqlite",
                    &toml_text(&RulesOut { rules: data.rules }, path)?,
                )?;
            }
        }
        knowledge_layers.push(layer);
    }
    let mut knowledge =
        KnowledgeBase::from_layers(knowledge_layers).map_err(WorkspaceError::from)?;
    for diagnostic in read_index_diagnostics(&connection, path)? {
        knowledge.add_diagnostic(diagnostic);
    }
    Ok(knowledge)
}

fn create_index_schema(connection: &Connection, path: &Utf8Path) -> Result<(), WorkspaceError> {
    connection
        .execute_batch(
            "
            DROP TABLE IF EXISTS knowledge_files;
            DROP TABLE IF EXISTS terms;
            DROP TABLE IF EXISTS memory;
            DROP TABLE IF EXISTS rules;
            DROP TABLE IF EXISTS diagnostics;
            CREATE TABLE knowledge_files(path TEXT PRIMARY KEY, layer TEXT NOT NULL, kind TEXT NOT NULL, fingerprint TEXT NOT NULL);
            CREATE TABLE terms(id TEXT PRIMARY KEY, layer TEXT NOT NULL, source TEXT NOT NULL, target TEXT NOT NULL, aliases_json TEXT NOT NULL, case_sensitive INTEGER NOT NULL, status TEXT NOT NULL, scope_json TEXT NOT NULL, tags_json TEXT NOT NULL, note TEXT);
            CREATE TABLE memory(id TEXT NOT NULL, layer TEXT NOT NULL, source TEXT NOT NULL, target TEXT NOT NULL, source_locale TEXT NOT NULL, target_locale TEXT NOT NULL, context_json TEXT NOT NULL, origin_json TEXT NOT NULL, quality TEXT NOT NULL, created_at TEXT, updated_at TEXT);
            CREATE TABLE rules(id TEXT PRIMARY KEY, layer TEXT NOT NULL, stage TEXT NOT NULL, mode TEXT NOT NULL, pattern TEXT NOT NULL, replacement TEXT NOT NULL, enabled INTEGER NOT NULL, scope_json TEXT NOT NULL, note TEXT);
            CREATE TABLE diagnostics(severity TEXT NOT NULL, code TEXT NOT NULL, message TEXT NOT NULL, entry_id TEXT NOT NULL, layer TEXT, rule_id TEXT);
            ",
        )
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })
}

fn write_terms(
    transaction: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for term in knowledge.terms() {
        transaction
            .execute(
                concat!(
                    "INSERT INTO terms(id, layer, source, target, aliases_json, case_sensitive, ",
                    "status, scope_json, tags_json, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
                ),
                params![
                    term.id(),
                    term.layer(),
                    term.source(),
                    term.target(),
                    json_string(term.aliases(), path)?,
                    term.case_sensitive(),
                    term.status().as_str(),
                    json_string(term.scope_values(), path)?,
                    json_string(term.tags(), path)?,
                    term.note(),
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn write_memory(
    transaction: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for item in knowledge.memory() {
        transaction
            .execute(
                concat!(
                    "INSERT INTO memory(id, layer, source, target, source_locale, target_locale, ",
                    "context_json, origin_json, quality, created_at, updated_at) ",
                    "VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
                ),
                params![
                    item.id(),
                    item.layer(),
                    item.source(),
                    item.target(),
                    item.source_locale(),
                    item.target_locale(),
                    json_string(item.context(), path)?,
                    json_string(item.origin(), path)?,
                    item.quality().as_str(),
                    item.created_at(),
                    item.updated_at(),
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn write_rules(
    transaction: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for rule in knowledge.rules() {
        transaction
            .execute(
                concat!(
                    "INSERT INTO rules(id, layer, stage, mode, pattern, replacement, enabled, ",
                    "scope_json, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
                ),
                params![
                    rule.id(),
                    rule.layer(),
                    rule.stage().as_str(),
                    rule.mode().as_str(),
                    rule.pattern(),
                    rule.replacement(),
                    rule.enabled(),
                    json_string(rule.scope_values(), path)?,
                    rule.note(),
                ],
            )
            .map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
    }
    Ok(())
}

fn write_diagnostics(
    transaction: &Connection,
    path: &Utf8Path,
    knowledge: &KnowledgeBase,
) -> Result<(), WorkspaceError> {
    for diagnostic in knowledge.merge_diagnostics() {
        transaction
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

fn read_indexed_files(
    connection: &Connection,
    path: &Utf8Path,
) -> Result<BTreeMap<String, (String, String, String)>, WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT path, layer, kind, fingerprint FROM knowledge_files")
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
                    row.get::<_, String>(3)?,
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

fn read_index_terms(
    connection: &Connection,
    path: &Utf8Path,
    layers: &mut BTreeMap<String, LayerData>,
) -> Result<(), WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT layer, id, source, target, aliases_json, case_sensitive, status, scope_json, tags_json, note FROM terms")
        .map_err(|source| WorkspaceError::Sqlite { path: path.to_owned(), source })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    for row in rows {
        let (layer, id, source, target, aliases, case_sensitive, status, scope, tags, note) =
            row.map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        let term = TermOut {
            id,
            source,
            target,
            aliases: json_value(aliases, path)?,
            case_sensitive,
            status,
            scope: json_value(scope, path)?,
            tags: json_value(tags, path)?,
            note,
        };
        layers.entry(layer).or_default().terms.push(term);
    }
    Ok(())
}

fn read_index_memory(
    connection: &Connection,
    path: &Utf8Path,
    layers: &mut BTreeMap<String, LayerData>,
) -> Result<(), WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT layer, id, source, target, source_locale, target_locale, context_json, origin_json, quality, created_at, updated_at FROM memory")
        .map_err(|source| WorkspaceError::Sqlite { path: path.to_owned(), source })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    for row in rows {
        let (
            layer,
            id,
            source,
            target,
            source_locale,
            target_locale,
            context,
            origin,
            quality,
            created_at,
            updated_at,
        ) = row.map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
        let memory = MemoryOut {
            id,
            source,
            target,
            source_locale,
            target_locale,
            context: json_value(context, path)?,
            origin: json_value(origin, path)?,
            quality,
            created_at,
            updated_at,
        };
        layers.entry(layer).or_default().memory.push(memory);
    }
    Ok(())
}

fn read_index_rules(
    connection: &Connection,
    path: &Utf8Path,
    layers: &mut BTreeMap<String, LayerData>,
) -> Result<(), WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT layer, id, stage, mode, pattern, replacement, enabled, scope_json, note FROM rules")
        .map_err(|source| WorkspaceError::Sqlite { path: path.to_owned(), source })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    for row in rows {
        let (layer, id, stage, mode, pattern, replacement, enabled, scope, note) =
            row.map_err(|source| WorkspaceError::Sqlite {
                path: path.to_owned(),
                source,
            })?;
        let rule = RuleOut {
            id,
            stage,
            mode,
            pattern,
            replacement,
            enabled,
            scope: json_value(scope, path)?,
            note,
        };
        layers.entry(layer).or_default().rules.push(rule);
    }
    Ok(())
}

fn read_index_diagnostics(
    connection: &Connection,
    path: &Utf8Path,
) -> Result<Vec<PipelineDiagnostic>, WorkspaceError> {
    let mut statement = connection
        .prepare("SELECT severity, code, message, entry_id, layer, rule_id FROM diagnostics")
        .map_err(|source| WorkspaceError::Sqlite {
            path: path.to_owned(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            let severity = severity_from_name(&row.get::<_, String>(0)?);
            let mut diagnostic = PipelineDiagnostic::new(
                severity,
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

#[derive(Debug, Default)]
struct LayerData {
    terms: Vec<TermOut>,
    memory: Vec<MemoryOut>,
    rules: Vec<RuleOut>,
}

#[derive(Debug, Serialize)]
struct TermsOut {
    terms: Vec<TermOut>,
}

#[derive(Debug, Serialize)]
struct TermOut {
    id: String,
    source: String,
    target: String,
    aliases: Vec<String>,
    case_sensitive: bool,
    status: String,
    scope: BTreeMap<String, Vec<String>>,
    tags: Vec<String>,
    note: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemoryOut {
    id: String,
    source: String,
    target: String,
    source_locale: String,
    target_locale: String,
    context: BTreeMap<String, String>,
    origin: Value,
    quality: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct RulesOut {
    rules: Vec<RuleOut>,
}

#[derive(Debug, Serialize)]
struct RuleOut {
    id: String,
    stage: String,
    mode: String,
    pattern: String,
    replacement: String,
    enabled: bool,
    scope: BTreeMap<String, Vec<String>>,
    note: Option<String>,
}

fn json_string(
    value: &(impl Serialize + ?Sized),
    path: &Utf8Path,
) -> Result<String, WorkspaceError> {
    serde_json::to_string(value).map_err(|source| WorkspaceError::Json {
        path: path.to_owned(),
        source,
    })
}

fn toml_text(value: &impl Serialize, path: &Utf8Path) -> Result<String, WorkspaceError> {
    toml::to_string(value).map_err(|source| WorkspaceError::Toml {
        path: path.to_owned(),
        source,
    })
}

fn json_value<T: DeserializeOwned>(text: String, path: &Utf8Path) -> Result<T, WorkspaceError> {
    serde_json::from_str(&text).map_err(|source| WorkspaceError::Json {
        path: path.to_owned(),
        source,
    })
}

fn file_kind_name(kind: KnowledgeFileKind) -> &'static str {
    match kind {
        KnowledgeFileKind::Terms => "terms",
        KnowledgeFileKind::Memory => "memory",
        KnowledgeFileKind::Rules => "rules",
    }
}

fn severity_from_name(value: &str) -> PipelineDiagnosticSeverity {
    match value {
        "error" => PipelineDiagnosticSeverity::Error,
        "info" => PipelineDiagnosticSeverity::Info,
        _ => PipelineDiagnosticSeverity::Warning,
    }
}
