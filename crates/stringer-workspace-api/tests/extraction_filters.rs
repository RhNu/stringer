use stringer_extraction_filter::{ExtractionFilterConfig, ExtractionFilterSet};
use stringer_workspace_api::{ExportTranslationsOptions, export_translations};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn export_applies_configured_extraction_filters_from_settings() {
    let root = TempRoot::new("export-configured-extraction-filters");
    let source_root = root.path().join("source");
    write_pex_fixture_with_literals(
        &source_root.join("Data/Scripts/Example.pex"),
        &["SomeIdentifier", "Open Door", "DEBUG skip"],
    );
    let export_path = root.path().join("translations");
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "pex.identifier_like_source"
enabled = false

[[rules]]
id = "user.skip_debug"
when = { field = "text", op = "contains", value = "DEBUG" }
"#,
    )
    .unwrap();
    let mut settings = settings();
    settings.extraction_filters = ExtractionFilterSet::from_config(config).unwrap();

    let summary = export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&export_path),
        settings,
        force: false,
    })
    .await
    .unwrap();

    assert_eq!(summary.entries, 2);
    let rows = entry_rows(&export_path, "pex", None);
    let sources = rows
        .iter()
        .map(|row| row["source"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(sources, ["SomeIdentifier", "Open Door"]);
}
