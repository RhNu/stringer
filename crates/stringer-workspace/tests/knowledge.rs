use std::fs;

use serde_json::Value;
use stringer_workspace::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, ExportTranslationsOptions,
    ImportTranslationsOptions, KnowledgeLayerOverrides, LoadWorkspaceSettingsOptions,
    LookupKnowledgeField, LookupKnowledgeMode, LookupKnowledgeOptions, LookupKnowledgeSource,
    PipelineEntryKind, ValidateTranslationsOptions, WorkspaceSettings, WorkspaceSettingsOverrides,
    WriteTarget, annotate_translations, build_knowledge_index, export_translations,
    import_translations, load_workspace_settings, lookup_knowledge, validate_translations,
};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn annotate_translations_writes_annotations_without_bumping_schema() {
    let root = TempRoot::new("annotate-package");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    write_text(
        &root.path().join("knowledge/terms/skyrim.toml"),
        r#"
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
status = "preferred"
scope = { game = "SkyrimSe" }
"#,
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
    })
    .await
    .unwrap();

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    assert_eq!(summary.entries, 1);
    assert_eq!(summary.auto_filled, 0);
    let manifest = json_file(&translations.join("manifest.json"));
    assert_eq!(manifest["schema_version"], 2);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
    assert_eq!(rows[0]["hints"][0]["kind"], "term");
    assert!(rows[0]["hints"][0].get("processor").is_none());
    assert_eq!(rows[0]["hints"][0]["payload"]["target"], "铁剑");
}

#[tokio::test]
async fn annotate_translations_removes_stale_diagnostics() {
    let root = TempRoot::new("annotate-clears-diagnostics");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
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

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 0);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("diagnostics").is_none());
}

#[tokio::test]
async fn annotate_translations_auto_fills_missing_memory_but_preserves_existing_translation() {
    let root = TempRoot::new("annotate-memory");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSteel Sword\n",
    );
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        concat!(
            "{\"id\":\"tm:1\",\"source\":\"Iron Sword\",\"target\":\"铁剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
            "{\"id\":\"tm:2\",\"source\":\"Steel Sword\",\"target\":\"钢剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\",\"created_at\":\"2026-05-14T00:00:00Z\"}\n",
        ),
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\",\"translation\":\"手工铁剑\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\"}\n",
        ),
    );

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: true,
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

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
    assert_eq!(desc["translation"], "钢剑");
}

#[tokio::test]
async fn validate_translations_recomputes_diagnostics_from_current_knowledge() {
    let root = TempRoot::new("validate-package");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tDragonborn\n",
    );
    write_text(
        &root.path().join("knowledge/terms/skyrim.toml"),
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
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
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

    let summary = validate_translations(ValidateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

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
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
    })
    .await
    .unwrap();

    let summary = validate_translations(ValidateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["diagnostics"][0]["code"], "translation.empty");
    assert!(rows[0]["diagnostics"][0].get("entry_id").is_none());
}

#[tokio::test]
async fn import_ignores_annotations_and_diagnostics() {
    let root = TempRoot::new("import-ignore-pipeline-metadata");
    let source = root
        .path()
        .join("Data/Interface/Translations/MyMod_English.txt");
    write_text(&source, "$Title\tIron Sword\n");
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings(),
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",",
            "\"translation\":\"铁剑\",",
            "\"hints\":[{\"kind\":\"term\",\"id\":\"x\",\"layer\":\"project\",\"confidence\":1.0,\"match\":\"source\"}],",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"term.preferred_missing\",\"message\":\"x\"}]}\n",
        ),
    );
    let override_root = TempRoot::new("import-ignore-pipeline-metadata-override");

    let summary = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
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
fn load_settings_defaults_global_knowledge_root_to_config_directory() {
    let root = TempRoot::new("settings-global-knowledge");
    let config = root.path().join("config/stringer.toml");
    write_text(
        &config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        config_path: Some(utf8(&config)),
        overrides: WorkspaceSettingsOverrides::default(),
    })
    .unwrap();

    assert_eq!(
        settings.global_knowledge_root,
        Some(utf8(&root.path().join("config").join("knowledge")))
    );
}

#[test]
fn lookup_uses_global_library_project_and_override_layers_in_order() {
    let root = TempRoot::new("knowledge-layer-order");
    let global = root.path().join("global-knowledge");
    let override_root = root.path().join("override-knowledge");
    write_term(
        &global.join("terms/base.toml"),
        "skyrim.weapon.iron_sword",
        "全局铁剑",
    );
    write_term(
        &global.join("libraries/SkyrimSe/zh-Hans/terms/library.toml"),
        "skyrim.weapon.iron_sword",
        "库铁剑",
    );
    write_term(
        &root.path().join("knowledge/terms/project.toml"),
        "skyrim.weapon.iron_sword",
        "项目铁剑",
    );
    write_term(
        &override_root.join("terms/override.toml"),
        "skyrim.weapon.iron_sword",
        "覆盖铁剑",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: vec![("record_type".to_string(), "WEAP".to_string())],
        knowledge: KnowledgeLayerOverrides {
            global_root: Some(utf8(&global)),
            override_root: Some(utf8(&override_root)),
        },
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(lookup.results.iter().any(|result| {
        result.kind == "term" && result.layer == "override" && result.target == "覆盖铁剑"
    }));
    assert!(lookup.diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "knowledge.override"
            && diagnostic.rule_id() == Some("skyrim.weapon.iron_sword")
    }));
}

#[test]
fn build_index_creates_sqlite_and_lookup_marks_fresh_index_used() {
    let root = TempRoot::new("knowledge-index-fresh");
    write_term(
        &root.path().join("knowledge/terms/project.toml"),
        "skyrim.weapon.iron_sword",
        "铁剑",
    );

    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();
    assert_eq!(summary.files, 1);
    assert_eq!(summary.terms, 1);
    assert!(
        root.path()
            .join(".stringer/indexes/knowledge.sqlite")
            .exists()
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        knowledge: KnowledgeLayerOverrides::default(),
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
fn lookup_falls_back_to_files_and_reports_stale_index_when_knowledge_changes() {
    let root = TempRoot::new("knowledge-index-stale");
    let terms = root.path().join("knowledge/terms/project.toml");
    write_term(&terms, "skyrim.weapon.iron_sword", "铁剑");
    build_knowledge_index(BuildKnowledgeIndexOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();
    write_term(&terms, "skyrim.weapon.iron_sword", "熟铁剑");

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        knowledge: KnowledgeLayerOverrides::default(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(!lookup.index_used);
    assert!(
        lookup
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "knowledge.index_stale")
    );
    assert_eq!(lookup.results[0].target, "熟铁剑");
}

#[tokio::test]
async fn annotate_reports_missing_index_as_knowledge_diagnostic_without_row_diagnostic() {
    let root = TempRoot::new("annotate-missing-index");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    write_term(
        &root.path().join("knowledge/terms/project.toml"),
        "skyrim.weapon.iron_sword",
        "铁剑",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings_with_global(None),
    })
    .await
    .unwrap();

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 0);
    assert!(
        summary
            .knowledge_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "knowledge.index_stale")
    );
}

#[tokio::test]
async fn annotate_uses_project_config_default_library_knowledge() {
    let root = TempRoot::new("annotate-project-config-library");
    write_text(
        &root.path().join("stringer.toml"),
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    write_term(
        &root
            .path()
            .join("knowledge/libraries/SkyrimSe/zh-Hans/terms/library.toml"),
        "skyrim.weapon.iron_sword",
        "库铁剑",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings_with_global(None),
    })
    .await
    .unwrap();

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    assert_eq!(summary.annotations, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["hints"][0]["layer"], "library");
    assert_eq!(rows[0]["hints"][0]["payload"]["target"], "库铁剑");
}

#[test]
fn lookup_falls_back_to_files_when_index_is_corrupt() {
    let root = TempRoot::new("knowledge-index-corrupt");
    write_term(
        &root.path().join("knowledge/terms/project.toml"),
        "skyrim.weapon.iron_sword",
        "铁剑",
    );
    write_bytes(
        &root.path().join(".stringer/indexes/knowledge.sqlite"),
        b"not sqlite",
    );

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        text: "Iron Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        knowledge: KnowledgeLayerOverrides::default(),
        mode: LookupKnowledgeMode::Contains,
        source: LookupKnowledgeSource::All,
        field: LookupKnowledgeField::Both,
        limit: 20,
        case_sensitive: false,
    })
    .unwrap();

    assert!(!lookup.index_used);
    assert!(
        lookup
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code() == "knowledge.index_stale")
    );
    assert_eq!(lookup.results[0].target, "铁剑");
}

#[test]
fn build_index_preserves_duplicate_memory_ids_across_layers() {
    let root = TempRoot::new("knowledge-index-memory-duplicates");
    let global = root.path().join("global-knowledge");
    write_text(
        &global.join("memory/base.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Iron Sword\",\"target\":\"全局铁剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );
    write_text(
        &root.path().join("knowledge/memory/project.jsonl"),
        "{\"id\":\"tm:1\",\"source\":\"Steel Sword\",\"target\":\"钢剑\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}\n",
    );

    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        knowledge: KnowledgeLayerOverrides {
            global_root: Some(utf8(&global)),
            override_root: None,
        },
    })
    .unwrap();

    assert_eq!(summary.memory, 2);
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
        root: utf8(root.path()),
        settings: settings_with_global(None),
        knowledge: KnowledgeLayerOverrides::default(),
    })
    .unwrap();

    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        root: utf8(root.path()),
        settings: settings_with_global(None),
        text: "Steel Sword".to_string(),
        kind: PipelineEntryKind::Plugin,
        context: Vec::new(),
        knowledge: KnowledgeLayerOverrides::default(),
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

#[tokio::test]
async fn annotate_reports_merge_diagnostics_once_as_knowledge_diagnostics() {
    let root = TempRoot::new("annotate-merge-diagnostics-once");
    let global = root.path().join("global-knowledge");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tIron Sword\n",
    );
    write_term(
        &global.join("terms/base.toml"),
        "skyrim.weapon.iron_sword",
        "全局铁剑",
    );
    write_term(
        &root.path().join("knowledge/terms/project.toml"),
        "skyrim.weapon.iron_sword",
        "项目铁剑",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: settings_with_global(None),
    })
    .await
    .unwrap();

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
        knowledge: KnowledgeLayerOverrides {
            global_root: Some(utf8(&global)),
            override_root: None,
        },
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 0);
    assert_eq!(
        summary
            .knowledge_diagnostics
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

fn settings_with_global(global_knowledge_root: Option<std::path::PathBuf>) -> WorkspaceSettings {
    let mut settings = settings();
    settings.global_knowledge_root = global_knowledge_root.as_deref().map(utf8);
    settings
}

fn write_term(path: &std::path::Path, id: &str, target: &str) {
    write_text(
        path,
        &format!(
            r#"
[[terms]]
id = "{id}"
source = "Iron Sword"
target = "{target}"
status = "preferred"
"#,
        ),
    );
}
