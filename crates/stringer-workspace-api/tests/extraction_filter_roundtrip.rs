use stringer_extraction_filter::{ExtractionFilterConfig, ExtractionFilterSet};
use stringer_workspace_api::{
    ExportTranslationsOptions, ImportTranslationsOptions, export_translations, import_translations,
};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn import_uses_exported_extraction_filters_for_roundtrip() {
    let root = TempRoot::new("extraction-filter-roundtrip");
    let source_root = root.path().join("source");
    write_pex_fixture_with_literals(
        &source_root.join("Data/Scripts/Example.pex"),
        &["SomeIdentifier", "Open Door"],
    );
    let export_path = root.path().join("translations");
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "pex.identifier_like_source"
enabled = false
"#,
    )
    .unwrap();
    let mut settings = settings();
    settings.extraction_filters = ExtractionFilterSet::from_config(config).unwrap();
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&export_path),
        settings,
        force: false,
    })
    .await
    .unwrap();
    write_entry_rows(
        &export_path,
        "pex",
        r#"{"id":"pex:Scripts/Example.pex:Example::Run:0:fixed-1:6","translation":"内部标识"}"#,
    );
    let output = root.path().join("override");

    let summary = import_translations(ImportTranslationsOptions {
        workspace: utf8(&export_path),
        source_root: None,
        output: utf8(&output),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(
        pex_entry_text(&output.join("Data/Scripts/Example.pex")),
        "内部标识"
    );
}
