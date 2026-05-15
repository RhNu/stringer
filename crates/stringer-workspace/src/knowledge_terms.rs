use std::collections::BTreeMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, Value, value};

use crate::WorkspaceError;
use crate::fsutil::{replace_file, temp_path};
use crate::knowledge::{BuildKnowledgeIndexOptions, KnowledgeIndexSummary, build_knowledge_index};
use crate::settings::WorkspaceSettings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTermUpsertOptions {
    pub project_root: Utf8PathBuf,
    pub file: Option<Utf8PathBuf>,
    pub term: KnowledgeTermInput,
    pub rebuild_index: bool,
    pub settings: Option<WorkspaceSettings>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTermDeleteOptions {
    pub project_root: Utf8PathBuf,
    pub file: Option<Utf8PathBuf>,
    pub id: String,
    pub rebuild_index: bool,
    pub settings: Option<WorkspaceSettings>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeTermInput {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub status: KnowledgeTermStatus,
    #[serde(default)]
    pub scope: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeTermStatus {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTermEditSummary {
    pub action: String,
    pub id: String,
    pub path: Utf8PathBuf,
    pub index_summary: Option<KnowledgeIndexSummary>,
}

pub fn upsert_knowledge_term(
    options: KnowledgeTermUpsertOptions,
) -> Result<KnowledgeTermEditSummary, WorkspaceError> {
    validate_scope(&options.term)?;
    let path = term_file_path(&options.project_root, options.file)?;
    let mut document = read_terms_document(&path)?;
    let id = options.term.id.clone();
    let table = term_table(&options.term);
    let terms = terms_array_mut(&mut document, &path)?;

    remove_matching_terms(terms, &id);
    terms.push(table);

    write_terms_document(&path, &document)?;
    let index_summary = rebuild_index_if_requested(
        options.rebuild_index,
        options.project_root,
        options.settings,
    )?;
    Ok(KnowledgeTermEditSummary {
        action: "upserted".to_string(),
        id,
        path,
        index_summary,
    })
}

pub fn delete_knowledge_term(
    options: KnowledgeTermDeleteOptions,
) -> Result<KnowledgeTermEditSummary, WorkspaceError> {
    let path = term_file_path(&options.project_root, options.file)?;
    let mut document = read_terms_document(&path)?;
    let terms = terms_array_mut(&mut document, &path)?;
    if remove_matching_terms(terms, &options.id) == 0 {
        return Err(WorkspaceError::KnowledgeTermNotFound {
            path,
            id: options.id,
        });
    }
    write_terms_document(&path, &document)?;
    let index_summary = rebuild_index_if_requested(
        options.rebuild_index,
        options.project_root,
        options.settings,
    )?;
    Ok(KnowledgeTermEditSummary {
        action: "deleted".to_string(),
        id: options.id,
        path,
        index_summary,
    })
}

fn term_file_path(
    project_root: &camino::Utf8Path,
    file: Option<Utf8PathBuf>,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let path = match file {
        Some(file) if file.is_relative() => project_root.join(file),
        Some(file) => file,
        None => project_root.join("knowledge/terms/project.toml"),
    };
    if path_is_in_project_terms(&path, project_root) {
        return Ok(path);
    }
    Err(WorkspaceError::InvalidKnowledgeTermFile {
        path,
        message: "term files must be .toml files under the project knowledge/terms directory"
            .to_string(),
    })
}

fn read_terms_document(path: &camino::Utf8Path) -> Result<DocumentMut, WorkspaceError> {
    if !path.exists() {
        return Ok(DocumentMut::new());
    }
    let text = fs::read_to_string(path).map_err(|source| WorkspaceError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    text.parse::<DocumentMut>()
        .map_err(|source| WorkspaceError::KnowledgeTermsToml {
            path: path.to_owned(),
            source: Box::new(source),
        })
}

fn write_terms_document(
    path: &camino::Utf8Path,
    document: &DocumentMut,
) -> Result<(), WorkspaceError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| WorkspaceError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let temp = temp_path(path, unique_temp_suffix());
    fs::write(&temp, document.to_string()).map_err(|source| WorkspaceError::WriteFile {
        path: temp.clone(),
        source,
    })?;
    replace_file(&temp, path)
}

fn terms_array_mut<'a>(
    document: &'a mut DocumentMut,
    path: &camino::Utf8Path,
) -> Result<&'a mut ArrayOfTables, WorkspaceError> {
    if !document.as_table().contains_key("terms") {
        document["terms"] = Item::ArrayOfTables(ArrayOfTables::new());
    }
    document["terms"]
        .as_array_of_tables_mut()
        .ok_or_else(|| invalid_terms(path, "`terms` must be an array of tables"))
}

fn remove_matching_terms(terms: &mut ArrayOfTables, id: &str) -> usize {
    let mut removed = 0;
    for index in (0..terms.len()).rev() {
        if terms
            .get(index)
            .and_then(|term| term.get("id"))
            .and_then(Item::as_str)
            == Some(id)
        {
            terms.remove(index);
            removed += 1;
        }
    }
    removed
}

fn term_table(input: &KnowledgeTermInput) -> Table {
    let mut table = Table::new();
    table["id"] = value(&input.id);
    table["source"] = value(&input.source);
    table["target"] = value(&input.target);
    if !input.aliases.is_empty() {
        table["aliases"] = string_array(&input.aliases);
    }
    table["case_sensitive"] = value(input.case_sensitive);
    table["status"] = value(input.status.as_str());
    if !input.scope.is_empty() {
        let mut scope = Table::new();
        for (key, values) in &input.scope {
            scope[key] = string_array(values);
        }
        table["scope"] = Item::Table(scope);
    }
    if !input.tags.is_empty() {
        table["tags"] = string_array(&input.tags);
    }
    if let Some(note) = &input.note {
        table["note"] = value(note);
    }
    table
}

fn string_array(values: &[String]) -> Item {
    let mut array = Array::new();
    for item in values {
        array.push(item.as_str());
    }
    Item::Value(Value::Array(array))
}

fn rebuild_index_if_requested(
    rebuild_index: bool,
    project_root: Utf8PathBuf,
    settings: Option<WorkspaceSettings>,
) -> Result<Option<KnowledgeIndexSummary>, WorkspaceError> {
    if !rebuild_index {
        return Ok(None);
    }
    let settings = settings.ok_or(WorkspaceError::MissingSetting { name: "settings" })?;
    build_knowledge_index(BuildKnowledgeIndexOptions {
        project_root,
        settings,
    })
    .map(Some)
}

fn validate_scope(input: &KnowledgeTermInput) -> Result<(), WorkspaceError> {
    for key in input.scope.keys() {
        if !SUPPORTED_SCOPE_KEYS.contains(&key.as_str()) {
            return Err(WorkspaceError::InvalidKnowledgeTermScope {
                id: input.id.clone(),
                key: key.clone(),
            });
        }
    }
    Ok(())
}

fn path_is_in_project_terms(path: &camino::Utf8Path, project_root: &camino::Utf8Path) -> bool {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("toml"))
    {
        return false;
    }
    let terms_root = project_root.join("knowledge/terms");
    let Some(path) = normalized_components(path) else {
        return false;
    };
    let Some(terms_root) = normalized_components(&terms_root) else {
        return false;
    };
    path.len() > terms_root.len() && path.starts_with(&terms_root)
}

fn normalized_components(path: &camino::Utf8Path) -> Option<Vec<String>> {
    let mut components = Vec::new();
    let normalized = path.as_str().replace('\\', "/");
    for component in normalized.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            value => components.push(value.to_lowercase()),
        }
    }
    Some(components)
}

fn unique_temp_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("terms-{nanos}")
}

fn invalid_terms(path: &camino::Utf8Path, message: impl Into<String>) -> WorkspaceError {
    WorkspaceError::InvalidKnowledgeTermsToml {
        path: path.to_owned(),
        message: message.into(),
    }
}

const SUPPORTED_SCOPE_KEYS: &[&str] = &[
    "game",
    "source_locale",
    "target_locale",
    "kind",
    "record_type",
    "asset_path",
];

impl KnowledgeTermStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Preferred => "preferred",
            Self::Allowed => "allowed",
            Self::Forbidden => "forbidden",
        }
    }
}
