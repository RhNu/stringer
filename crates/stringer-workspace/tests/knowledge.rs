use std::fs;

use serde_json::Value;
use stringer_workspace::{
    AnnotateTranslationsOptions, ExportTranslationsOptions, ImportTranslationsOptions,
    ValidateTranslationsOptions, WriteTarget, annotate_translations, export_translations,
    import_translations, validate_translations,
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
    })
    .unwrap();

    assert_eq!(summary.entries, 1);
    assert_eq!(summary.auto_filled, 0);
    let manifest = json_file(&translations.join("manifest.json"));
    assert_eq!(manifest["schema_version"], 2);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translated_text").is_none());
    assert_eq!(rows[0]["annotations"][0]["kind"], "term");
    assert_eq!(rows[0]["annotations"][0]["payload"]["target"], "铁剑");
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
            "\"source_text\":\"Iron Sword\",",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"stale\",\"message\":\"old\",\"entry_id\":\"old\"}]}\n",
        ),
    );

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: false,
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
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source_text\":\"Iron Sword\",\"translated_text\":\"手工铁剑\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source_text\":\"Steel Sword\"}\n",
        ),
    );

    let summary = annotate_translations(AnnotateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        allow_memory_auto_fill: true,
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
    assert_eq!(title["translated_text"], "手工铁剑");
    assert_eq!(desc["translated_text"], "钢剑");
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
            "\"source_text\":\"Dragonborn\",",
            "\"translated_text\":\"抓根宝\",",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"stale\",\"message\":\"old\",\"entry_id\":\"old\"}]}\n",
        ),
    );

    let summary = validate_translations(ValidateTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
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
async fn validate_translations_reports_missing_translated_text() {
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
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows[0]["diagnostics"][0]["code"], "translation.empty");
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
            "\"translated_text\":\"铁剑\",",
            "\"annotations\":[{\"kind\":\"term\",\"id\":\"x\",\"layer\":\"project\",\"confidence\":1.0,\"match\":\"source\",\"processor\":\"stringer.term\"}],",
            "\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"term.preferred_missing\",\"message\":\"x\",\"entry_id\":\"x\"}]}\n",
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

fn row_by_id<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["id"].as_str() == Some(id))
        .expect("row by id")
}
