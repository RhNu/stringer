use stringer_knowledge::{ValidateTranslationsOptions, validate_translations};
use stringer_workspace_api::{ExportTranslationsOptions, export_translations};
use stringer_workspace_core::GlobalConfigSource;

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn validate_translations_does_not_require_skipped_entries() {
    let root = TempRoot::new("validate-skipped-translation");
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
        "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\",\"translation_meta\":{\"origin\":\"skipped\",\"skip_reason\":\"not_translatable\"},\"diagnostics\":[{\"severity\":\"info\",\"code\":\"translation.empty\",\"message\":\"old\"}]}\n",
    );

    let summary = validate_translations(ValidateTranslationsOptions {
        workspace: utf8(&translations),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
    })
    .unwrap();

    assert_eq!(summary.diagnostics, 0);
    let rows = entry_rows(&translations, "scaleform", None);
    assert!(rows[0].get("translation").is_none());
    assert_eq!(rows[0]["translation_meta"]["origin"], "skipped");
    assert!(rows[0].get("diagnostics").is_none());
}
