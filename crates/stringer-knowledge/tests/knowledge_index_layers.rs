use rusqlite::Connection;
use stringer_knowledge::{
    BuildKnowledgeIndexOptions, KnowledgeIndexBuildScope, KnowledgeTermDeleteOptions,
    LoadKnowledgeLayersOptions, LookupKnowledgeField, LookupKnowledgeMode, LookupKnowledgeOptions,
    LookupKnowledgeSource, build_knowledge_index, delete_knowledge_term, load_knowledge_layers,
    lookup_knowledge,
};
use stringer_pipeline::PipelineEntryKind;
use stringer_workspace_core::{GlobalConfigSource, WorkspaceSettings};

#[allow(dead_code)]
mod support;

use support::*;

#[test]
fn explicit_rebuild_splits_global_and_workspace_indexes() {
    let root = TempRoot::new("knowledge-layered-index-rebuild");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    let workspace_term = root.path().join("knowledge/terms/workspace.toml");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");
    write_term(&workspace_term, "skyrim.weapon.iron_sword", "工作区铁剑");

    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();

    assert_eq!(summary.files, 2);
    assert_eq!(summary.terms, 1);
    assert_eq!(summary.memory, 1);
    assert!(global.join("index.sqlite").exists());
    assert!(root.path().join("knowledge/index.sqlite").exists());

    let global_files = indexed_source_files(&global.join("index.sqlite"));
    assert_eq!(global_files, vec![path_string(&global_memory)]);
    let workspace_files = indexed_source_files(&root.path().join("knowledge/index.sqlite"));
    assert_eq!(workspace_files, vec![path_string(&workspace_term)]);
}

#[test]
fn build_index_counts_duplicate_memory_ids_across_layers() {
    let root = TempRoot::new("knowledge-index-memory-duplicates");
    let global = root.path().join("global-knowledge");
    write_memory(
        &global.join("memory/base.jsonl"),
        "tm:1",
        "Iron Sword",
        "全局铁剑",
    );
    write_memory(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "tm:1",
        "Steel Sword",
        "钢剑",
    );

    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global)),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();

    assert_eq!(summary.memory, 2);
}

#[test]
fn lookup_auto_creates_global_index_without_copying_global_rows_to_workspace_index() {
    let root = TempRoot::new("knowledge-layered-index-lookup");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    let workspace_term = root.path().join("knowledge/terms/workspace.toml");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");
    write_term_with_source(
        &workspace_term,
        "skyrim.weapon.steel_sword",
        "Steel Sword",
        "工作区钢剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.index_used);
    assert_eq!(lookup.results[0].target, "全局铁剑");
    assert!(global.join("index.sqlite").exists());
    assert!(root.path().join("knowledge/index.sqlite").exists());
    assert_eq!(
        indexed_source_files(&global.join("index.sqlite")),
        vec![path_string(&global_memory)]
    );
    assert_eq!(
        indexed_source_files(&root.path().join("knowledge/index.sqlite")),
        vec![path_string(&workspace_term)]
    );
}

#[test]
fn lookup_with_empty_workspace_knowledge_uses_global_index_only() {
    let root = TempRoot::new("knowledge-layered-index-global-only");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.index_used);
    assert_eq!(lookup.results[0].target, "全局铁剑");
    assert!(global.join("index.sqlite").exists());
    assert!(!root.path().join("knowledge/index.sqlite").exists());
}

#[test]
fn lookup_with_empty_workspace_knowledge_files_uses_global_index_only() {
    let root = TempRoot::new("knowledge-layered-index-empty-workspace-files");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");
    write_text(
        &root.path().join("knowledge/terms/empty.toml"),
        "# no workspace terms\n",
    );
    write_text(
        &root.path().join("knowledge/rules/empty.toml"),
        "# no workspace rules\n",
    );
    write_text(&root.path().join("knowledge/memory/empty.jsonl"), "\n");

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.index_used);
    assert_eq!(lookup.results[0].target, "全局铁剑");
    assert!(global.join("index.sqlite").exists());
    assert!(!root.path().join("knowledge/index.sqlite").exists());
}

#[test]
fn lookup_reports_cross_layer_override_from_split_indexes() {
    let root = TempRoot::new("knowledge-layered-index-override");
    let global = root.path().join("global-knowledge");
    write_term(
        &global.join("terms/base.toml"),
        "skyrim.weapon.iron_sword",
        "全局铁剑",
    );
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "工作区铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert_eq!(lookup.results[0].target, "工作区铁剑");
    assert_eq!(lookup.total_matches, 1);
    assert!(lookup.results.iter().all(|result| result.layer != "global"));
    assert_eq!(
        lookup
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code() == "knowledge.override"
                    && diagnostic.rule_id() == Some("skyrim.weapon.iron_sword")
            })
            .count(),
        1
    );

    let hidden_global = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global)),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "全局铁剑".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert_eq!(hidden_global.total_matches, 0);
    assert!(hidden_global.results.is_empty());
}

#[test]
fn load_knowledge_layers_suppresses_overridden_global_memory() {
    let root = TempRoot::new("knowledge-layered-load-memory-override");
    let global = root.path().join("global-knowledge");
    write_memory(
        &global.join("memory/base.jsonl"),
        "tm:iron",
        "Iron Sword",
        "全局铁剑",
    );
    write_memory(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "tm:iron",
        "Iron Sword",
        "工作区铁剑",
    );

    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global)),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        prefer_index: false,
    })
    .unwrap();

    assert_eq!(loaded.knowledge.memory().len(), 1);
    assert_eq!(loaded.knowledge.memory()[0].layer(), "workspace");
    assert_eq!(loaded.knowledge.memory()[0].target(), "工作区铁剑");
    assert_eq!(
        loaded
            .knowledge
            .merge_diagnostics()
            .iter()
            .filter(|diagnostic| {
                diagnostic.code() == "knowledge.override" && diagnostic.rule_id() == Some("tm:iron")
            })
            .count(),
        1
    );
}

#[test]
fn lookup_suppresses_overridden_global_memory_from_split_indexes() {
    let root = TempRoot::new("knowledge-layered-index-memory-override");
    let global = root.path().join("global-knowledge");
    write_memory(
        &global.join("memory/base.jsonl"),
        "tm:iron",
        "Iron Sword",
        "全局铁剑",
    );
    write_memory(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "tm:iron",
        "Iron Sword",
        "工作区铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global)),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert_eq!(lookup.total_matches, 1);
    assert_eq!(lookup.results[0].kind, "memory");
    assert_eq!(lookup.results[0].layer, "workspace");
    assert_eq!(lookup.results[0].target, "工作区铁剑");
    assert_eq!(
        lookup
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code() == "knowledge.override" && diagnostic.rule_id() == Some("tm:iron")
            })
            .count(),
        1
    );
}

#[test]
fn lookup_suppresses_diagnostics_from_overridden_global_rule() {
    let root = TempRoot::new("knowledge-layered-index-rule-override");
    let global = root.path().join("global-knowledge");
    write_rule(
        &global.join("rules/base.toml"),
        "protect.player_name",
        "[",
        "regex",
    );
    write_rule(
        &root.path().join("knowledge/rules/workspace.toml"),
        "protect.player_name",
        "{PLAYER_NAME}",
        "literal",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global)),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.diagnostics.iter().all(|diagnostic| {
        !(diagnostic.code() == "replacement.regex_invalid"
            && diagnostic.rule_id() == Some("protect.player_name"))
    }));
    assert_eq!(
        lookup
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code() == "knowledge.override"
                    && diagnostic.rule_id() == Some("protect.player_name")
            })
            .count(),
        1
    );
}

#[test]
fn term_delete_rebuilds_workspace_index_without_creating_global_index() {
    let root = TempRoot::new("knowledge-layered-index-term-delete");
    let global = root.path().join("global-knowledge");
    let workspace_terms = root.path().join("knowledge/terms/workspace.toml");
    write_memory(
        &global.join("memory/base.jsonl"),
        "tm:iron",
        "Iron Sword",
        "全局铁剑",
    );
    write_term(&workspace_terms, "skyrim.weapon.iron_sword", "工作区铁剑");

    let summary = delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace: utf8(root.path()),
        file: None,
        id: "skyrim.weapon.iron_sword".to_string(),
        rebuild_index: true,
        settings: Some(settings_with_global(Some(global.clone()))),
    })
    .unwrap();

    assert!(summary.index_summary.is_some());
    assert!(root.path().join("knowledge/index.sqlite").exists());
    assert!(!global.join("index.sqlite").exists());
}

#[test]
fn term_delete_workspace_rebuild_does_not_walk_global_root() {
    let root = TempRoot::new("knowledge-layered-index-term-delete-no-global-walk");
    let global = root.path().join("global-knowledge");
    let workspace_terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(&global.join("terms"), "not a directory");
    write_term(&workspace_terms, "skyrim.weapon.iron_sword", "工作区铁剑");

    let summary = delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace: utf8(root.path()),
        file: None,
        id: "skyrim.weapon.iron_sword".to_string(),
        rebuild_index: true,
        settings: Some(settings_with_global(Some(global.clone()))),
    })
    .unwrap();

    assert!(summary.index_summary.is_some());
    assert!(root.path().join("knowledge/index.sqlite").exists());
    assert!(!global.join("index.sqlite").exists());
}

#[test]
fn lookup_rebuilds_corrupt_global_index_without_moving_global_rows_to_workspace_index() {
    let root = TempRoot::new("knowledge-layered-index-corrupt-global");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    let workspace_term = root.path().join("knowledge/terms/workspace.toml");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");
    write_term_with_source(
        &workspace_term,
        "skyrim.weapon.steel_sword",
        "Steel Sword",
        "工作区钢剑",
    );
    build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();
    insert_index_meta(
        &root.path().join("knowledge/index.sqlite"),
        "test_sentinel",
        "workspace-reused",
    );
    write_bytes(&global.join("index.sqlite"), b"not sqlite");

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert_eq!(lookup.results[0].target, "全局铁剑");
    assert_eq!(
        indexed_source_files(&global.join("index.sqlite")),
        vec![path_string(&global_memory)]
    );
    assert_eq!(
        indexed_source_files(&root.path().join("knowledge/index.sqlite")),
        vec![path_string(&workspace_term)]
    );
    assert_eq!(
        index_meta_value(&root.path().join("knowledge/index.sqlite"), "test_sentinel"),
        Some("workspace-reused".to_string())
    );
}

#[test]
fn lookup_rebuilds_stale_workspace_index_without_rebuilding_global_index() {
    let root = TempRoot::new("knowledge-layered-index-stale-workspace");
    let global = root.path().join("global-knowledge");
    let global_memory = global.join("memory/base.jsonl");
    let workspace_term = root.path().join("knowledge/terms/workspace.toml");
    write_memory(&global_memory, "tm:iron", "Iron Sword", "全局铁剑");
    write_term_with_source(
        &workspace_term,
        "skyrim.weapon.steel_sword",
        "Steel Sword",
        "工作区钢剑",
    );
    build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();
    insert_index_meta(
        &global.join("index.sqlite"),
        "test_sentinel",
        "global-reused",
    );
    write_term_with_source(
        &workspace_term,
        "skyrim.weapon.steel_sword",
        "Steel Sword",
        "工作区精钢剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(Some(global.clone())),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Steel Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert_eq!(lookup.results[0].target, "工作区精钢剑");
    assert_eq!(
        index_meta_value(&global.join("index.sqlite"), "test_sentinel"),
        Some("global-reused".to_string())
    );
}

fn settings_with_global(global_knowledge_root: Option<std::path::PathBuf>) -> WorkspaceSettings {
    let mut settings = settings();
    settings.global_knowledge_root = Some(match global_knowledge_root {
        Some(path) => utf8(&path),
        None => camino::Utf8PathBuf::from("__stringer_test_no_global_knowledge__"),
    });
    settings
}

fn indexed_source_files(path: &std::path::Path) -> Vec<String> {
    let connection = Connection::open(path).unwrap();
    let mut statement = connection
        .prepare("SELECT path FROM source_files ORDER BY path")
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| row.unwrap().replace('\\', "/"))
        .collect()
}

fn path_string(path: &std::path::Path) -> String {
    utf8(path).to_string().replace('\\', "/")
}

fn insert_index_meta(path: &std::path::Path, key: &str, value: &str) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute("INSERT INTO meta(key, value) VALUES (?1, ?2)", [key, value])
        .unwrap();
}

fn index_meta_value(path: &std::path::Path, key: &str) -> Option<String> {
    let connection = Connection::open(path).unwrap();
    connection
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .ok()
}

fn write_term(path: &std::path::Path, id: &str, target: &str) {
    write_term_with_source(path, id, "Iron Sword", target);
}

fn write_term_with_source(path: &std::path::Path, id: &str, source: &str, target: &str) {
    write_text(
        path,
        &format!(
            r#"
[[terms]]
id = "{id}"
source = "{source}"
target = "{target}"
status = "preferred"
"#,
        ),
    );
}

fn write_memory(path: &std::path::Path, id: &str, source: &str, target: &str) {
    write_text(
        path,
        &format!(
            "{{\"id\":\"{id}\",\"source\":\"{source}\",\"target\":\"{target}\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}}\n"
        ),
    );
}

fn write_rule(path: &std::path::Path, id: &str, pattern: &str, mode: &str) {
    write_text(
        path,
        &format!(
            r#"
[[rules]]
id = "{id}"
stage = "pre_translate"
pattern = "{pattern}"
replacement = "__TOKEN__"
mode = "{mode}"
enabled = true
"#,
        ),
    );
}
