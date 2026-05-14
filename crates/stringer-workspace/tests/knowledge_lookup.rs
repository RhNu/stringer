use stringer_workspace::{
    KnowledgeLayerOverrides, LookupKnowledgeField, LookupKnowledgeMode, LookupKnowledgeOptions,
    LookupKnowledgeSource, PipelineEntryKind, WorkspaceSettings, lookup_knowledge,
};

#[allow(dead_code)]
mod support;

use support::*;

#[test]
fn lookup_searches_terms_and_memory_source_and_target_by_default() {
    let root = TempRoot::new("lookup-search-defaults");
    write_text(
        &root.path().join("knowledge/terms/project.toml"),
        r#"
[[terms]]
id = "term:altmer"
source = "Altmer"
target = "高精灵"
status = "preferred"
"#,
    );
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:altmer-1\",\"source\":\"The Altmer Embassy\",\"target\":\"梭默大使馆\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"context\":{\"record_type\":\"CELL\"}}\n",
            "{\"id\":\"tm:target-1\",\"source\":\"High Elf\",\"target\":\"Altmer 传统\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"imported\"}\n",
        ),
    );

    let lookup = lookup_knowledge(lookup_options(root.path(), "altmer")).unwrap();

    assert_eq!(lookup.query, "altmer");
    assert_eq!(lookup.mode, LookupKnowledgeMode::Contains);
    assert_eq!(lookup.total_matches, 3);
    assert_eq!(lookup.results[0].id, "term:altmer");
    assert_eq!(lookup.results[0].kind, "term");
    assert_eq!(lookup.results[0].match_field, "source");
    assert_eq!(lookup.results[0].match_kind, "exact");
    assert!(lookup.results.iter().any(|result| {
        result.kind == "memory"
            && result.id == "tm:target-1"
            && result.match_field == "target"
            && result.target == "Altmer 传统"
    }));
}

#[test]
fn lookup_limit_caps_results_but_reports_total_matches() {
    let root = TempRoot::new("lookup-limit");
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:1\",\"source\":\"Altmer\",\"target\":\"高精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
            "{\"id\":\"tm:2\",\"source\":\"Altmer Armor\",\"target\":\"高精灵护甲\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
            "{\"id\":\"tm:3\",\"source\":\"Altmer Robes\",\"target\":\"高精灵长袍\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
        ),
    );
    let mut options = lookup_options(root.path(), "altmer");
    options.limit = 2;

    let lookup = lookup_knowledge(options).unwrap();

    assert_eq!(lookup.total_matches, 3);
    assert_eq!(lookup.results.len(), 2);
}

#[test]
fn lookup_supports_regex_mode_and_reports_invalid_regex() {
    let root = TempRoot::new("lookup-regex");
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:altmer\",\"source\":\"Altmer\",\"target\":\"高精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
            "{\"id\":\"tm:bosmer\",\"source\":\"Bosmer\",\"target\":\"木精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
        ),
    );
    let mut options = lookup_options(root.path(), "^(Alt|Bos)mer$");
    options.mode = LookupKnowledgeMode::Regex;

    let lookup = lookup_knowledge(options).unwrap();

    assert_eq!(lookup.total_matches, 2);
    assert!(lookup.results.iter().all(|result| result.kind == "memory"));

    let mut invalid = lookup_options(root.path(), "[");
    invalid.mode = LookupKnowledgeMode::Regex;
    let error = lookup_knowledge(invalid).unwrap_err().to_string();
    assert!(error.contains("invalid lookup regex"));
}

#[test]
fn lookup_loads_nested_adapt_memory_files() {
    let root = TempRoot::new("lookup-nested-adapt-memory");
    write_text(
        &root.path().join("knowledge/memory/adapt/skyrim.esm.jsonl"),
        "{\"id\":\"tm:altmer\",\"source\":\"Altmer\",\"target\":\"高精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );

    let lookup = lookup_knowledge(lookup_options(root.path(), "altmer")).unwrap();

    assert_eq!(lookup.total_matches, 1);
    assert_eq!(lookup.results[0].id, "tm:altmer");
    assert_eq!(lookup.results[0].target, "高精灵");
}

#[test]
fn lookup_omits_rejected_memory_by_default() {
    let root = TempRoot::new("lookup-rejected-memory");
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:accepted\",\"source\":\"Altmer\",\"target\":\"高精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
            "{\"id\":\"tm:rejected\",\"source\":\"Altmer\",\"target\":\"错误译名\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"rejected\"}\n",
        ),
    );

    let lookup = lookup_knowledge(lookup_options(root.path(), "altmer")).unwrap();

    assert_eq!(lookup.total_matches, 1);
    assert_eq!(lookup.results[0].id, "tm:accepted");
}

#[test]
fn lookup_context_conflict_does_not_outrank_context_match() {
    let root = TempRoot::new("lookup-context-conflict");
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:conflict\",\"source\":\"Altmer\",\"target\":\"高精灵\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"context\":{\"record_type\":\"ARMO\"}}\n",
            "{\"id\":\"tm:match\",\"source\":\"Altmer Armor\",\"target\":\"高精灵护甲\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"context\":{\"record_type\":\"WEAP\"}}\n",
        ),
    );

    let lookup = lookup_knowledge(lookup_options(root.path(), "altmer")).unwrap();

    assert_eq!(lookup.total_matches, 2);
    assert_eq!(lookup.results[0].id, "tm:match");
}

fn lookup_options(root: &std::path::Path, text: &str) -> LookupKnowledgeOptions {
    LookupKnowledgeOptions {
        root: utf8(root),
        settings: settings_with_global(None),
        text: text.to_string(),
        kind: PipelineEntryKind::Plugin,
        context: vec![("record_type".to_string(), "WEAP".to_string())],
        knowledge: KnowledgeLayerOverrides::default(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    }
}

fn settings_with_global(global_knowledge_root: Option<std::path::PathBuf>) -> WorkspaceSettings {
    let mut settings = settings();
    settings.global_knowledge_root = global_knowledge_root.as_deref().map(utf8);
    settings
}
