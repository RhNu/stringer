use std::fs;

use stringer_workspace::{
    ExportTranslationsOptions, ImportTranslationsOptions, export_translations, import_translations,
};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn open_allows_existing_workspace_inputs_without_force() {
    let root = TempRoot::new("open-allows-inputs");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let workspace = root.path().join("workspace");
    write_text(
        &workspace.join("stringer.toml"),
        "target_locale = \"zh-Hans\"\n",
    );
    write_text(
        &workspace.join("knowledge/terms/workspace.toml"),
        r#"
[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "铁剑"
"#,
    );
    fs::create_dir_all(workspace.join("knowledge/memory")).unwrap();
    fs::create_dir_all(workspace.join("knowledge/rules")).unwrap();

    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&workspace),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    assert!(workspace.join("workspace.json").exists());
    assert!(workspace.join("stringer.toml").exists());
    assert!(workspace.join("knowledge/terms/workspace.toml").exists());
}

#[tokio::test]
async fn open_rejects_generated_artifacts_and_unknown_paths_without_force() {
    let root = TempRoot::new("open-rejects-existing");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let generated = root.path().join("generated-workspace");
    write_text(&generated.join("workspace.json"), "{}");
    let error = export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&generated),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains("generated artifact"));

    let unknown = root.path().join("unknown-workspace");
    write_text(&unknown.join("notes.txt"), "keep");
    let error = export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&unknown),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains("unknown existing path"));
}

#[tokio::test]
async fn force_open_replaces_generated_artifacts_and_preserves_workspace_inputs() {
    let root = TempRoot::new("force-open-replaces-generated");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let workspace = root.path().join("workspace");
    write_text(&workspace.join("workspace.json"), "{}");
    write_text(&workspace.join("entries/stale.jsonl"), "{}\n");
    write_text(&workspace.join("batches/stale.json"), "{}");
    write_text(&workspace.join("knowledge/index.sqlite"), "stale");
    write_text(
        &workspace.join("knowledge/terms/workspace.toml"),
        r#"
[[terms]]
id = "term:iron_sword"
source = "Iron Sword"
target = "铁剑"
"#,
    );
    write_text(
        &workspace.join("stringer.toml"),
        "target_locale = \"zh-Hans\"\n",
    );
    write_text(&workspace.join("output/readme.txt"), "preserve");

    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&workspace),
        settings: settings(),
        force: true,
    })
    .await
    .unwrap();

    assert_eq!(
        json_file(&workspace.join("workspace.json"))["schema_version"],
        4
    );
    assert!(!workspace.join("entries/stale.jsonl").exists());
    assert!(!workspace.join("batches/stale.json").exists());
    assert!(!workspace.join("knowledge/index.sqlite").exists());
    assert!(workspace.join("knowledge/terms/workspace.toml").exists());
    assert!(workspace.join("stringer.toml").exists());
    assert_eq!(
        fs::read_to_string(workspace.join("output/readme.txt")).unwrap(),
        "preserve"
    );
}

#[tokio::test]
async fn force_open_validates_source_before_replacing_generated_artifacts() {
    let root = TempRoot::new("force-open-source-first");
    let workspace = root.path().join("workspace");
    write_text(&workspace.join("workspace.json"), "{}");
    write_text(&workspace.join("entries/stale.jsonl"), "{}\n");

    let error = export_translations(ExportTranslationsOptions {
        source_root: utf8(&root.path().join("missing-source")),
        workspace: utf8(&workspace),
        settings: settings(),
        force: true,
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("failed to read"));
    assert!(workspace.join("workspace.json").exists());
    assert!(workspace.join("entries/stale.jsonl").exists());
}

#[tokio::test]
async fn force_open_does_not_replace_generated_artifacts_when_workspace_is_locked() {
    let root = TempRoot::new("force-open-locked");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let workspace = root.path().join("workspace");
    write_text(&workspace.join("lock"), "{}");
    write_text(&workspace.join("workspace.json"), "{}");
    write_text(&workspace.join("entries/stale.jsonl"), "{}\n");

    let error = export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&workspace),
        settings: settings(),
        force: true,
    })
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        stringer_workspace::WorkspaceError::WorkspaceLocked { .. }
    ));
    assert!(workspace.join("workspace.json").exists());
    assert!(workspace.join("entries/stale.jsonl").exists());
}

#[tokio::test]
async fn open_rejects_workspace_inside_source_root() {
    let root = TempRoot::new("open-rejects-source-workspace-overlap");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );

    let error = export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&source_root),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("workspace must be outside the source root")
    );
    assert!(!source_root.join("workspace.json").exists());
}

#[tokio::test]
async fn import_uses_stored_source_root_and_allows_source_root_override() {
    let root = TempRoot::new("import-source-root");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tWood\n",
    );
    let override_source_root = root.path().join("override-source");
    write_text(
        &override_source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSteel\n",
    );
    let workspace = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&workspace),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &workspace,
        "scaleform",
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":"铁剑"}"#,
    );

    let stored_output = root.path().join("stored-output");
    import_translations(ImportTranslationsOptions {
        workspace: utf8(&workspace),
        source_root: None,
        output: utf8(&stored_output),
    })
    .await
    .unwrap();
    let stored =
        fs::read(stored_output.join("Data/Interface/Translations/MyMod_English.txt")).unwrap();
    assert!(decode_utf16le_bom(&stored).contains("$Desc\tWood\n"));

    let override_output = root.path().join("override-output");
    import_translations(ImportTranslationsOptions {
        workspace: utf8(&workspace),
        source_root: Some(utf8(&override_source_root)),
        output: utf8(&override_output),
    })
    .await
    .unwrap();
    let overridden =
        fs::read(override_output.join("Data/Interface/Translations/MyMod_English.txt")).unwrap();
    assert!(decode_utf16le_bom(&overridden).contains("$Desc\tSteel\n"));
}
