use std::fs;

use serde_json::Value;
use stringer_core::Language;
use stringer_plugin::{GameRelease, StringsFile, StringsKind, write_strings_file};
use stringer_workspace::{
    ExportTranslationsOptions, ImportTranslationsOptions, LoadWorkspaceSettingsOptions,
    WorkspaceSettings, WorkspaceSettingsOverrides, WriteTarget, export_translations,
    import_translations, load_workspace_settings,
};

mod support;

use support::*;

#[test]
fn load_settings_reads_toml_config_and_applies_explicit_overrides() {
    let root = TempRoot::new("settings");
    let config = root.path().join("config.toml");
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
        overrides: WorkspaceSettingsOverrides {
            target_locale: Some("ja".to_string()),
            ..WorkspaceSettingsOverrides::default()
        },
    })
    .unwrap();

    assert_eq!(settings.game_release, GameRelease::SkyrimSe);
    assert_eq!(settings.asset_language, Language::English);
    assert_eq!(settings.source_locale, "en");
    assert_eq!(settings.target_locale, "ja");
}

#[tokio::test]
async fn exports_scaleform_package_with_manifest_and_clean_entry_rows() {
    let root = TempRoot::new("export-scaleform");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let out = root.path().join("translations");

    let summary = export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&out),
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.entries, 1);
    let manifest = json_file(&out.join("manifest.json"));
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(manifest["game_release"], "SkyrimSe");
    assert_eq!(manifest["asset_language"], "English");
    assert_eq!(manifest["source_locale"], "en");
    assert_eq!(manifest["target_locale"], "zh-Hans");
    assert_eq!(manifest["files"].as_array().unwrap().len(), 1);
    assert_eq!(
        manifest["files"][0]["path"],
        "entries/scaleform/Interface/Translations/MyMod_English.txt.jsonl"
    );
    assert_eq!(manifest["files"][0]["kind"], "scaleform");
    assert_eq!(
        manifest["files"][0]["asset_path"],
        "Interface/Translations/MyMod_English.txt"
    );
    assert!(manifest["files"][0].get("group").is_none());

    let rows = jsonl_rows(
        &out.join(
            manifest["files"][0]["path"]
                .as_str()
                .expect("entry file path"),
        ),
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["id"],
        "scaleform:Interface/Translations/MyMod_English.txt:$Title"
    );
    assert_eq!(rows[0]["source"], "Iron Sword");
    assert_eq!(rows[0]["context"]["key"], "$Title");
    assert!(rows[0].get("schema_version").is_none());
    assert!(rows[0].get("kind").is_none());
    assert!(rows[0].get("asset_path").is_none());
    assert!(rows[0].get("asset_language").is_none());
    assert!(rows[0].get("source_locale").is_none());
    assert!(rows[0].get("target_locale").is_none());
}

#[tokio::test]
async fn import_writes_only_changed_override_files_and_leaves_source_unchanged() {
    let root = TempRoot::new("import-scaleform");
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
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":"钢剑"}"#,
    );
    let override_root = TempRoot::new("import-scaleform-override");

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
    assert_eq!(summary.written_files, 1);
    assert_eq!(fs::read_to_string(&source).unwrap(), "$Title\tIron Sword\n");
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t钢剑\n"));
}

#[tokio::test]
async fn import_uses_asset_language_from_manifest() {
    let root = TempRoot::new("manifest-language");
    let source = root
        .path()
        .join("Data/Interface/Translations/MyMod_French.txt");
    write_text(&source, "$Title\tEpee en fer\n");
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&translations),
        settings: WorkspaceSettings {
            game_release: GameRelease::SkyrimSe,
            asset_language: Language::French,
            source_locale: "fr".to_string(),
            target_locale: "zh-Hans".to_string(),
            global_knowledge_root: None,
        },
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        r#"{"id":"scaleform:Interface/Translations/MyMod_French.txt:$Title","translation":"铁剑"}"#,
    );
    let override_root = TempRoot::new("manifest-language-override");

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
    assert_eq!(summary.written_files, 1);
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_French.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t铁剑\n"));
}

#[tokio::test]
async fn import_rejects_duplicate_translated_ids() {
    let root = TempRoot::new("duplicate-id");
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
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"translation\":\"A\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"translation\":\"B\"}\n",
        ),
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("duplicate translation id"));
}

#[tokio::test]
async fn import_rejects_duplicate_ids_even_when_only_one_row_has_translation() {
    let root = TempRoot::new("duplicate-id-missing-text");
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
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"translation\":\"Steel Sword\"}\n",
        ),
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("duplicate translation id"));
}

#[tokio::test]
async fn import_rejects_override_root_that_is_the_source_root() {
    let root = TempRoot::new("unsafe-override-root");
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
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":"Steel Sword"}"#,
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(root.path()),
        },
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("override root"));
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("Data/Interface/Translations/MyMod_English.txt")
        )
        .unwrap(),
        "$Title\tIron Sword\n"
    );
}

#[tokio::test]
async fn import_rejects_unknown_translation_ids() {
    let root = TempRoot::new("unknown-id");
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
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Missing","translation":"Steel Sword"}"#,
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
    })
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("does not match any exported entry")
    );
}

#[tokio::test]
async fn import_rejects_unsupported_translation_package_schema_version() {
    let root = TempRoot::new("bad-schema");
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
    let mut manifest = json_file(&translations.join("manifest.json"));
    manifest["schema_version"] = Value::from(999);
    write_text(
        &translations.join("manifest.json"),
        &serde_json::to_string(&manifest).unwrap(),
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
    })
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unsupported translation package schema version")
    );
}

#[tokio::test]
async fn import_rejects_manifest_entry_paths_that_escape_package_root() {
    let root = TempRoot::new("unsafe-package-path");
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
    let mut manifest = json_file(&translations.join("manifest.json"));
    manifest["files"][0]["path"] = Value::from("../outside.jsonl");
    write_text(
        &translations.join("manifest.json"),
        &serde_json::to_string(&manifest).unwrap(),
    );

    let error = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
    })
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid translation package path")
    );
}

#[tokio::test]
async fn import_ignores_unlisted_translation_package_entry_files() {
    let root = TempRoot::new("ignore-unlisted");
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
    write_text(
        &translations.join("entries/scaleform/unlisted.jsonl"),
        r#"{"id":"scaleform:Missing.txt:$Title","translation":"Bad"}"#,
    );
    let override_root = TempRoot::new("ignore-unlisted-override");

    let summary = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 0);
    assert_eq!(summary.written_files, 0);
}

#[tokio::test]
async fn import_skips_missing_and_null_translation_without_writing_files() {
    let root = TempRoot::new("skip-null");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSharp blade\n",
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
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"translation\":null}\n",
        ),
    );
    let override_root = TempRoot::new("skip-null-override");

    let summary = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 0);
    assert_eq!(summary.written_files, 0);
    assert!(!override_root.path().join("Data").exists());
}

#[tokio::test]
async fn import_applies_empty_string_translations() {
    let root = TempRoot::new("empty-string");
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
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":""}"#,
    );
    let override_root = TempRoot::new("empty-string-override");

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
    assert_eq!(summary.written_files, 1);
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t\n"));
}

#[tokio::test]
async fn import_same_text_translation_does_not_write_override_files() {
    let root = TempRoot::new("same-text");
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
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":"Iron Sword"}"#,
    );
    let override_root = TempRoot::new("same-text-override");

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
    assert_eq!(summary.written_files, 0);
    assert!(!override_root.path().join("Data").exists());
}

#[tokio::test]
async fn exported_ids_normalize_only_path_separators_not_key_text() {
    let root = TempRoot::new("id-normalization");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Path\\Key\tIron Sword\n",
    );
    let out = root.path().join("translations");

    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&out),
        settings: settings(),
    })
    .await
    .unwrap();

    let rows = entry_rows(&out, "scaleform", None);
    assert_eq!(
        rows[0]["id"],
        "scaleform:Interface/Translations/MyMod_English.txt:$Path\\Key"
    );
}

#[tokio::test]
async fn import_updates_localized_plugin_strings_without_copying_unchanged_plugin() {
    let root = TempRoot::new("plugin-import");
    write_bytes(
        &root.path().join("Data/MyMod.esp"),
        &build_localized_plugin(),
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let strings_asset = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &strings,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    write_bytes(
        &root.path().join("Data/Strings/MyMod_English.STRINGS"),
        strings_asset.bytes(),
    );

    let export_path = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();
    let manifest = json_file(&export_path.join("manifest.json"));
    let plugin_file = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["kind"] == "plugin")
        .expect("plugin file");
    assert_eq!(plugin_file["path"], "entries/plugin/MyMod.esp/WEAP.jsonl");
    assert_eq!(plugin_file["asset_path"], "MyMod.esp");
    assert_eq!(plugin_file["group"], "WEAP");
    let rows = entry_rows(&export_path, "plugin", Some("WEAP"));
    let plugin_row = rows
        .iter()
        .find(|row| {
            row["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("plugin:"))
        })
        .expect("plugin row");
    assert!(
        plugin_row["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("plugin:MyMod.esp:"))
    );
    write_entry_rows(
        &export_path,
        "plugin",
        &format!(
            "{{\"id\":{},\"translation\":\"钢剑\"}}\n",
            serde_json::to_string(plugin_row["id"].as_str().unwrap()).unwrap()
        ),
    );
    let override_root = TempRoot::new("plugin-import-override");

    let summary = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&export_path),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    assert!(!override_root.path().join("Data/MyMod.esp").exists());
    assert!(
        override_root
            .path()
            .join("Data/Strings/MyMod_English.STRINGS")
            .exists()
    );
}

#[tokio::test]
async fn export_omits_pex_files_with_only_filtered_sources() {
    let root = TempRoot::new("pex-export-filtered");
    write_pex_fixture_with_literals(
        &root.path().join("Data/Scripts/Example.pex"),
        &["", "SomeIdentifier", "tag,tag,tag"],
    );
    let export_path = root.path().join("translations");

    let summary = export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.entries, 0);
    let manifest = json_file(&export_path.join("manifest.json"));
    assert!(
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["kind"] != "pex")
    );
}

#[tokio::test]
async fn export_keeps_only_unfiltered_pex_sources() {
    let root = TempRoot::new("pex-export-mixed");
    write_pex_fixture_with_literals(
        &root.path().join("Data/Scripts/Example.pex"),
        &["SomeIdentifier", "Open Door", "tag,tag,tag", "Hello world"],
    );
    let export_path = root.path().join("translations");

    let summary = export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.entries, 2);
    let rows = entry_rows(&export_path, "pex", None);
    let sources = rows
        .iter()
        .map(|row| row["source"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(sources, ["Open Door", "Hello world"]);
}

#[tokio::test]
async fn import_updates_pex_literals_into_override_script() {
    let root = TempRoot::new("pex-import");
    let source = root.path().join("Data/Scripts/Example.pex");
    write_pex_fixture(&source);
    let export_path = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();
    let manifest = json_file(&export_path.join("manifest.json"));
    let pex_file = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["kind"] == "pex")
        .expect("pex file");
    assert_eq!(pex_file["path"], "entries/pex/Scripts/Example.pex.jsonl");
    assert_eq!(pex_file["asset_path"], "Scripts/Example.pex");
    let rows = entry_rows(&export_path, "pex", None);
    let pex_row = rows
        .iter()
        .find(|row| row["id"].as_str().is_some_and(|id| id.starts_with("pex:")))
        .expect("pex row");
    assert!(
        pex_row["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("pex:Scripts/Example.pex:"))
    );
    write_entry_rows(
        &export_path,
        "pex",
        &format!(
            "{{\"id\":{},\"translation\":\"你好\"}}\n",
            serde_json::to_string(pex_row["id"].as_str().unwrap()).unwrap()
        ),
    );
    let override_root = TempRoot::new("pex-import-override");

    let summary = import_translations(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&export_path),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    assert_eq!(
        pex_entry_text(&override_root.path().join("Data/Scripts/Example.pex")),
        "你好"
    );
    assert_eq!(fs::read(source).unwrap(), pex_fixture_bytes());
}
