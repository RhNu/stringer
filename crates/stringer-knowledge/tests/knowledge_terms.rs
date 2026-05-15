use std::fs;

use camino::Utf8PathBuf;
use stringer_knowledge::{
    KnowledgeError, KnowledgeTermDeleteOptions, KnowledgeTermInput, KnowledgeTermStatus,
    KnowledgeTermUpsertOptions, KnowledgeTermsUpsertOptions, LoadKnowledgeLayersOptions,
    delete_knowledge_term, load_knowledge_layers, upsert_knowledge_term, upsert_knowledge_terms,
};

#[allow(dead_code)]
mod support;

use support::*;

#[test]
fn term_upsert_creates_default_workspace_file_and_loads_from_knowledge_layers() {
    let root = TempRoot::new("term-upsert-create");

    let summary = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    assert_eq!(summary.action, "upserted");
    assert_eq!(summary.id, "term:iron_sword");
    assert!(summary.path.ends_with("knowledge/terms/workspace.toml"));
    assert!(summary.index_summary.is_none());

    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: utf8(root.path()),
        settings: settings(),
        prefer_index: false,
    })
    .unwrap();
    let term = loaded
        .knowledge
        .terms()
        .iter()
        .find(|term| term.id() == "term:iron_sword")
        .unwrap();

    assert_eq!(term.source(), "Iron Sword");
    assert_eq!(term.target(), "熟铁剑");
    assert_eq!(term.status(), stringer_pipeline::TermStatus::Preferred);
}

#[test]
fn term_upsert_replaces_existing_term_without_removing_unrelated_terms_or_file_comments() {
    let root = TempRoot::new("term-upsert-replace");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(
        &terms,
        r#"# workspace terminology

[[terms]]
id = "term:keep"
source = "Steel Sword"
target = "钢剑"

[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "旧铁剑"
aliases = ["old iron"]
status = "allowed"

[terms.scope]
kind = ["plugin"]
"#,
    );

    let mut term = iron_sword_term("熟铁剑");
    term.aliases = vec!["Iron Blade".to_string(), "iron weapon".to_string()];
    term.status = KnowledgeTermStatus::Forbidden;
    term.scope
        .insert("game".to_string(), vec!["SkyrimSe".to_string()]);
    term.scope
        .insert("kind".to_string(), vec!["plugin".to_string()]);
    term.tags = vec!["weapon".to_string()];
    term.note = Some("Use the workspace-specific wording.".to_string());

    upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        term,
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    let text = fs::read_to_string(&terms).unwrap();
    assert!(text.contains("# workspace terminology"));
    assert!(text.contains("term:keep"));
    assert_eq!(text.matches("term:iron_sword").count(), 1);
    assert!(!text.contains("旧铁剑"));

    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: utf8(root.path()),
        settings: settings(),
        prefer_index: false,
    })
    .unwrap();
    let term = loaded
        .knowledge
        .terms()
        .iter()
        .find(|term| term.id() == "term:iron_sword")
        .unwrap();

    assert_eq!(term.target(), "熟铁剑");
    assert_eq!(term.aliases(), ["Iron Blade", "iron weapon"]);
    assert_eq!(term.status(), stringer_pipeline::TermStatus::Forbidden);
    assert_eq!(
        term.scope_values().get("game").unwrap(),
        &vec!["SkyrimSe".to_string()]
    );
    assert_eq!(term.tags(), ["weapon"]);
    assert_eq!(term.note(), Some("Use the workspace-specific wording."));
}

#[test]
fn term_delete_removes_matching_term_and_reports_missing_ids() {
    let root = TempRoot::new("term-delete");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(
        &terms,
        r#"
[[terms]]
id = "term:keep"
source = "Steel Sword"
target = "钢剑"

[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "熟铁剑"
"#,
    );

    let summary = delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace: utf8(root.path()),
        file: None,
        id: "term:iron_sword".to_string(),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    assert_eq!(summary.action, "deleted");
    assert_eq!(summary.id, "term:iron_sword");
    assert!(
        !fs::read_to_string(&terms)
            .unwrap()
            .contains("term:iron_sword")
    );

    let error = delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace: utf8(root.path()),
        file: None,
        id: "term:missing".to_string(),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::KnowledgeTermNotFound { id, .. } if id == "term:missing"
    ));
}

#[test]
fn term_upsert_replaces_all_duplicate_ids_with_one_managed_term() {
    let root = TempRoot::new("term-upsert-duplicate-ids");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(
        &terms,
        r#"
[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "旧铁剑"

[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "重复铁剑"
"#,
    );

    upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    let text = fs::read_to_string(&terms).unwrap();
    assert_eq!(text.matches("term:iron_sword").count(), 1);
    assert!(text.contains("熟铁剑"));
    assert!(!text.contains("旧铁剑"));
    assert!(!text.contains("重复铁剑"));
}

#[test]
fn terms_upsert_replaces_existing_terms_and_appends_new_terms() {
    let root = TempRoot::new("terms-upsert-batch");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(
        &terms,
        r#"# workspace terminology

[[terms]]
id = "term:keep"
source = "Dagger"
target = "匕首"

[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "旧铁剑"

[[terms]]
id = "term:steel_sword"
source = "Steel Sword"
target = "旧钢剑"
"#,
    );

    let mut steel = iron_sword_term("钢剑");
    steel.id = "term:steel_sword".to_string();
    steel.source = "Steel Sword".to_string();

    let summary = upsert_knowledge_terms(KnowledgeTermsUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        terms: vec![iron_sword_term("熟铁剑"), steel],
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    assert_eq!(summary.action, "upserted");
    assert_eq!(summary.ids, ["term:iron_sword", "term:steel_sword"]);
    assert_eq!(summary.count, 2);
    assert!(summary.index_summary.is_none());

    let text = fs::read_to_string(&terms).unwrap();
    assert!(text.contains("# workspace terminology"));
    assert!(text.contains("term:keep"));
    assert_eq!(text.matches("term:iron_sword").count(), 1);
    assert_eq!(text.matches("term:steel_sword").count(), 1);
    assert!(!text.contains("旧铁剑"));
    assert!(!text.contains("旧钢剑"));

    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: utf8(root.path()),
        settings: settings(),
        prefer_index: false,
    })
    .unwrap();
    assert_eq!(loaded.knowledge.terms().len(), 3);
}

#[test]
fn term_delete_removes_all_duplicate_ids() {
    let root = TempRoot::new("term-delete-duplicate-ids");
    let terms = root.path().join("knowledge/terms/workspace.toml");
    write_text(
        &terms,
        r#"
[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "旧铁剑"

[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "重复铁剑"
"#,
    );

    delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace: utf8(root.path()),
        file: None,
        id: "term:iron_sword".to_string(),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap();

    assert!(
        !fs::read_to_string(&terms)
            .unwrap()
            .contains("term:iron_sword")
    );
}

#[test]
fn term_upsert_rejects_unsupported_scope_keys() {
    let root = TempRoot::new("term-upsert-invalid-scope");
    let mut term = iron_sword_term("熟铁剑");
    term.scope
        .insert("subrecord".to_string(), vec!["FULL".to_string()]);

    let error = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        term,
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::InvalidKnowledgeTermScope { key, .. } if key == "subrecord"
    ));
}

#[test]
fn term_upsert_rejects_file_overrides_outside_workspace_terms_root() {
    let root = TempRoot::new("term-upsert-outside-terms-root");

    let error = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: Some(Utf8PathBuf::from("weapons.toml")),
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::InvalidKnowledgeTermFile { .. }
    ));
}

#[test]
fn term_upsert_rejects_file_overrides_that_traverse_outside_workspace_terms_root() {
    let root = TempRoot::new("term-upsert-traversal");

    let error = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: Some(Utf8PathBuf::from(
            "knowledge/terms/../memory/workspace.toml",
        )),
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::InvalidKnowledgeTermFile { .. }
    ));
}

#[test]
fn term_upsert_rejects_file_overrides_without_toml_extension() {
    let root = TempRoot::new("term-upsert-non-toml");

    let error = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: Some(Utf8PathBuf::from("knowledge/terms/weapons.txt")),
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::InvalidKnowledgeTermFile { .. }
    ));
}

#[test]
fn term_upsert_rejects_terms_root_as_file_override() {
    let root = TempRoot::new("term-upsert-terms-root");

    let error = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: Some(Utf8PathBuf::from("knowledge/terms")),
        term: iron_sword_term("熟铁剑"),
        rebuild_index: false,
        settings: Some(settings()),
    })
    .unwrap_err();

    assert!(matches!(
        error,
        KnowledgeError::InvalidKnowledgeTermFile { .. }
    ));
}

#[test]
fn term_upsert_can_rebuild_knowledge_index() {
    let root = TempRoot::new("term-upsert-rebuild-index");

    let summary = upsert_knowledge_term(KnowledgeTermUpsertOptions {
        workspace: utf8(root.path()),
        file: None,
        term: iron_sword_term("熟铁剑"),
        rebuild_index: true,
        settings: Some(settings()),
    })
    .unwrap();

    assert_eq!(summary.index_summary.unwrap().terms, 1);
    assert!(root.path().join("knowledge/index.sqlite").exists());

    let loaded = load_knowledge_layers(LoadKnowledgeLayersOptions {
        workspace: utf8(root.path()),
        settings: settings(),
        prefer_index: true,
    })
    .unwrap();

    assert!(loaded.index_used);
    assert_eq!(loaded.knowledge.terms()[0].target(), "熟铁剑");
}

fn iron_sword_term(target: &str) -> KnowledgeTermInput {
    KnowledgeTermInput {
        id: "term:iron_sword".to_string(),
        source: "Iron Sword".to_string(),
        target: target.to_string(),
        aliases: Vec::new(),
        case_sensitive: false,
        status: KnowledgeTermStatus::Preferred,
        scope: Default::default(),
        tags: Vec::new(),
        note: None,
    }
}
