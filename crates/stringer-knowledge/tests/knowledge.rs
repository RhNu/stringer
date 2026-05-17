use std::fs;

use serde_json::Value;
use stringer_knowledge::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeIndexBuildScope,
    LookupKnowledgeField, LookupKnowledgeMode, LookupKnowledgeOptions, LookupKnowledgeSource,
    ValidateTranslationsOptions, annotate_translations, build_knowledge_index, lookup_knowledge,
    validate_translations,
};
use stringer_pipeline::PipelineEntryKind;
use stringer_workspace_api::{
    ClaimBatchOptions, ExportTranslationsOptions, ImportTranslationsOptions, claim_batch,
    export_translations, import_translations,
};
use stringer_workspace_core::{GlobalConfigSource, WorkspaceSettings};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn annotate_translations_writes_annotations_without_bumping_schema() {
    let root = TempRoot::new("annotate-package");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/terms/skyrim.toml"),
        r#"
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
status = "preferred"
scope = { game = "SkyrimSe" }
"#,
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    let summary = annotate_translations(annotate_options(&translations, true)).unwrap();

    assert_eq!(summary.entries, 1);
    assert_eq!(summary.auto_filled, 0);
    let workspace = json_file(&translations.join("workspace.json"));
    assert_eq!(workspace["schema_version"], 4);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
    assert_eq!(rows[0]["hints"][0]["kind"], "term");
    assert!(rows[0]["hints"][0].get("processor").is_none());
    assert_eq!(rows[0]["hints"][0]["payload"]["target"], "铁剑");
}

#[tokio::test]
async fn annotate_translations_preserves_existing_diagnostics() {
    let root = TempRoot::new("annotate-preserves-diagnostics");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",",
            "\"source\":\"Iron Sword\",",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"stale\",\"message\":\"old\"}]}\n",
        ),
    );

    let summary = annotate_translations(annotate_options(&translations, true)).unwrap();

    assert_eq!(summary.diagnostics, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["diagnostics"][0]["code"], "stale");
}

#[tokio::test]
async fn annotate_translations_fills_missing_memory_by_default_but_preserves_existing_translation()
{
    let root = TempRoot::new("annotate-memory");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n$Desc\tStringer Test Steel Source\n",
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/memory/workspace.jsonl"),
        concat!(
            "{\"id\":\"tm:1\",\"source\":\"Stringer Test Iron Source\",\"target\":\"测试铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
            "{\"id\":\"tm:2\",\"source\":\"Stringer Test Steel Source\",\"target\":\"测试钢源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
        ),
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Stringer Test Iron Source\",\"translation\":\"手工铁剑\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Stringer Test Steel Source\"}\n",
        ),
    );

    let summary = annotate_translations(annotate_options(&translations, false)).unwrap();

    assert_eq!(summary.auto_filled, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    let title = row_by_id(
        &rows,
        "scaleform:Interface/Translations/MyMod_English.txt:$Title",
    );
    let desc = row_by_id(
        &rows,
        "scaleform:Interface/Translations/MyMod_English.txt:$Desc",
    );
    assert_eq!(title["translation"], "手工铁剑");
    assert_eq!(desc["translation"], "测试钢源");
    assert!(title.get("translation_meta").is_none());
    assert_eq!(desc["translation_meta"]["origin"], "memory");
    assert!(desc["translation_meta"]["updated_at_unix_ms"].is_number());
}

#[tokio::test]
async fn annotate_translations_does_not_fill_agent_origin_entries() {
    let root = TempRoot::new("annotate-agent-origin");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n",
    );
    write_text(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Stringer Test Iron Source\",\"target\":\"测试铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Stringer Test Iron Source\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
    );

    let summary = annotate_translations(annotate_options(&translations, false)).unwrap();

    assert_eq!(summary.auto_filled, 0);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
    assert_eq!(rows[0]["translation_meta"]["origin"], "agent");
}

#[tokio::test]
async fn annotate_translations_does_not_fill_claimed_entries() {
    let root = TempRoot::new("annotate-claimed-entry");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n",
    );
    write_text(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Stringer Test Iron Source\",\"target\":\"测试铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: None,
        limit: 1,
    })
    .unwrap();
    assert!(claim.batch_id.is_some());

    let summary = annotate_translations(annotate_options(&translations, false)).unwrap();

    assert_eq!(summary.auto_filled, 0);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
}

#[tokio::test]
async fn annotate_translations_can_skip_memory_fill() {
    let root = TempRoot::new("annotate-skip-memory-fill");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n",
    );
    write_text(
        &root.path().join("knowledge/memory/workspace.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Stringer Test Iron Source\",\"target\":\"测试铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    let summary = annotate_translations(annotate_options(&translations, true)).unwrap();

    assert_eq!(summary.auto_filled, 0);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
}

#[tokio::test]
async fn annotate_uses_indexed_workspace_memory_for_auto_fill() {
    let root = TempRoot::new("annotate-indexed-workspace-memory");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n",
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/memory/workspace.jsonl"),
        "{\"id\":\"tm:iron\",\"source\":\"Stringer Test Iron Source\",\"target\":\"工作区铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    let summary = annotate_translations(annotate_options(&translations, false)).unwrap();

    assert_eq!(summary.auto_filled, 1);
    assert!(summary.index_used);
    assert!(summary.knowledge_diagnostics.is_empty());
    assert!(translations.join("knowledge/index.sqlite").exists());
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["translation"], "工作区铁源");
    assert_eq!(rows[0]["hints"][0]["layer"], "workspace");
}

#[tokio::test]
async fn validate_translations_recomputes_diagnostics_from_current_knowledge() {
    let root = TempRoot::new("validate-package");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tDragonborn\n",
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/terms/skyrim.toml"),
        r#"
[[terms]]
id = "skyrim.dragonborn.preferred"
source = "Dragonborn"
target = "龙裔"
status = "preferred"

[[terms]]
id = "skyrim.dragonborn.forbidden"
source = "Dragonborn"
target = "抓根宝"
status = "forbidden"
"#,
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",",
            "\"source\":\"Dragonborn\",",
            "\"translation\":\"抓根宝\",",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"stale\",\"message\":\"old\"}]}\n",
        ),
    );

    let summary = validate_translations(validate_options(&translations)).unwrap();

    assert_eq!(summary.entries, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    let codes = rows[0]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"term.forbidden_used"));
    assert!(codes.contains(&"term.preferred_missing"));
    assert!(!codes.contains(&"stale"));
}

#[tokio::test]
async fn validate_translations_reports_missing_translation() {
    let root = TempRoot::new("validate-missing-translation");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    let summary = validate_translations(validate_options(&translations)).unwrap();

    assert_eq!(summary.diagnostics, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["diagnostics"][0]["code"], "translation.empty");
    assert!(rows[0]["diagnostics"][0].get("entry_id").is_none());
}

#[tokio::test]
async fn validate_uses_indexed_rejected_memory_for_conflict_diagnostics() {
    let root = TempRoot::new("validate-indexed-rejected-memory");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tStringer Test Iron Source\n",
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/memory/workspace.jsonl"),
        "{\"id\":\"tm:bad\",\"source\":\"Stringer Test Iron Source\",\"target\":\"错误铁源\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"rejected\"}\n",
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Stringer Test Iron Source\",\"translation\":\"错误铁源\"}\n",
    );
    let summary = validate_translations(validate_options(&translations)).unwrap();
    assert!(summary.index_used);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(
        rows[0]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic["code"] == "memory.conflict" && diagnostic["rule_id"] == "tm:bad"
            })
    );
}

#[tokio::test]
async fn import_ignores_annotations_and_diagnostics() {
    let root = TempRoot::new("import-ignore-pipeline-metadata");
    let source_root = root.path().join("source");
    let source = source_root.join("Data/Interface/Translations/MyMod_English.txt");
    write_text(&source, "$Title\tIron Sword\n");
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",",
            "\"translation\":\"铁剑\",",
            "\"hints\":[{\"kind\":\"term\",\"id\":\"x\",\"layer\":\"workspace\",\"confidence\":1.0,\"match\":\"source\"}],",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"term.preferred_missing\",\"message\":\"x\"}]}\n",
        ),
    );
    let override_root = TempRoot::new("import-ignore-pipeline-metadata-override");

    let summary = import_translations(ImportTranslationsOptions {
        workspace: utf8(&translations),
        source_root: None,
        output: utf8(override_root.path()),
        force: true,
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t铁剑\n"));
}

#[test]
fn lookup_uses_global_then_workspace_layers_and_ignores_library_fixtures() {
    let root = TempRoot::new("knowledge-layer-order");
    let global = root.path().join("global-knowledge");
    write_term(
        &global.join("terms/base.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "全局铁剑",
    );
    write_term(
        &global.join("libraries/SkyrimSe/zh-Hans/terms/library.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "库铁剑",
    );
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "工作区铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings(),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(Some(utf8(&global))),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: vec![("record_type".to_string(), "WEAP".to_string())],
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.results.iter().any(|result| {
        result.kind == "term" && result.layer == "workspace" && result.target == "工作区铁剑"
    }));
    assert!(
        lookup
            .results
            .iter()
            .all(|result| result.target != "库铁剑")
    );
    assert!(lookup.diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "knowledge.override"
            && diagnostic.rule_id() == Some("skyrim.weapon.iron_sword")
    }));
}

#[test]
fn build_index_creates_sqlite_and_lookup_marks_fresh_index_used() {
    let root = TempRoot::new("knowledge-index-fresh");
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Codex Fresh Index Iron Sword",
        "铁剑",
    );

    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();
    assert!(summary.files >= 1);
    assert!(summary.terms >= 1);
    assert!(root.path().join("knowledge/index.sqlite").exists());

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Codex Fresh Index Iron Sword".to_string(),
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
    assert!(lookup.diagnostics.is_empty());
    assert_eq!(lookup.results[0].target, "铁剑");
}

#[test]
fn lookup_auto_creates_missing_index_without_stale_diagnostic() {
    let root = TempRoot::new("knowledge-index-auto-create");
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Codex Auto Index Iron Sword",
        "铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Codex Auto Index Iron Sword".to_string(),
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
    assert!(lookup.diagnostics.is_empty());
    assert_eq!(lookup.results[0].target, "铁剑");
    assert!(root.path().join("knowledge/index.sqlite").exists());
}

#[test]
fn lookup_refreshes_changed_knowledge_index_without_stale_diagnostic() {
    let root = TempRoot::new("knowledge-index-refresh");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_term(
        &terms,
        "skyrim.weapon.iron_sword",
        "Codex Refresh Index Iron Sword",
        "铁剑",
    );
    build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();
    write_term(
        &terms,
        "skyrim.weapon.iron_sword",
        "Codex Refresh Index Iron Sword",
        "熟铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Codex Refresh Index Iron Sword".to_string(),
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
    assert!(lookup.diagnostics.is_empty());
    assert_eq!(lookup.results[0].target, "熟铁剑");
}

#[tokio::test]
async fn annotate_auto_refreshes_missing_index_without_stale_diagnostic() {
    let root = TempRoot::new("annotate-auto-refresh-index");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    write_term(
        &translations.join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "铁剑",
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings_with_global(None),
        force: false,
    })
    .await
    .unwrap();

    let summary = annotate_translations(annotate_options(&translations, true)).unwrap();

    assert_eq!(summary.diagnostics, 0);
    assert!(summary.index_used);
    assert!(
        summary
            .knowledge_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code() != "knowledge.index_stale")
    );
    assert!(translations.join("knowledge/index.sqlite").exists());
}

#[test]
fn lookup_rebuilds_corrupt_index_without_stale_diagnostic() {
    let root = TempRoot::new("knowledge-index-corrupt");
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Codex Corrupt Index Iron Sword",
        "铁剑",
    );
    write_bytes(&root.path().join("knowledge/index.sqlite"), b"not sqlite");

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        text: "Codex Corrupt Index Iron Sword".to_string(),
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
    assert!(lookup.diagnostics.is_empty());
    assert_eq!(lookup.results[0].target, "铁剑");
}

#[test]
fn fresh_index_preserves_duplicate_memory_ids_across_files_in_same_layer() {
    let root = TempRoot::new("knowledge-index-memory-duplicates-same-layer");
    write_text(
        &root.path().join("knowledge/memory/first.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Iron Sword\",\"target\":\"铁剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );
    write_text(
        &root.path().join("knowledge/memory/second.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Steel Sword\",\"target\":\"钢剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );
    build_knowledge_index(BuildKnowledgeIndexOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        scope: KnowledgeIndexBuildScope::All,
    })
    .unwrap();

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace: utf8(root.path()),
        settings: settings_with_global(None),
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

    assert!(lookup.index_used);
    assert!(
        lookup
            .results
            .iter()
            .any(|result| { result.kind == "memory" && result.target == "钢剑" })
    );
}

#[test]
fn lookup_reports_merge_diagnostics_once() {
    let root = TempRoot::new("lookup-merge-diagnostics-once");
    let global = root.path().join("global-knowledge");
    write_term(
        &global.join("terms/base.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "全局铁剑",
    );
    write_term(
        &root.path().join("knowledge/terms/workspace.toml"),
        "skyrim.weapon.iron_sword",
        "Iron Sword",
        "项目铁剑",
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

    assert_eq!(
        lookup
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code() == "knowledge.override")
            .count(),
        1
    );
}

fn row_by_id<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["id"].as_str() == Some(id))
        .expect("row by id")
}

fn annotate_options(
    workspace: &std::path::Path,
    skip_memory_fill: bool,
) -> AnnotateTranslationsOptions {
    AnnotateTranslationsOptions {
        workspace: utf8(workspace),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        skip_memory_fill,
    }
}

fn validate_options(workspace: &std::path::Path) -> ValidateTranslationsOptions {
    ValidateTranslationsOptions {
        workspace: utf8(workspace),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
    }
}

fn settings_with_global(global_knowledge_root: Option<std::path::PathBuf>) -> WorkspaceSettings {
    let mut settings = settings();
    settings.global_knowledge_root = Some(match global_knowledge_root {
        Some(path) => utf8(&path),
        None => camino::Utf8PathBuf::from("__stringer_test_no_global_knowledge__"),
    });
    settings
}

fn write_term(path: &std::path::Path, id: &str, source: &str, target: &str) {
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
