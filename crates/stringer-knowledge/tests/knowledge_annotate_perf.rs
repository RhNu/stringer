use stringer_knowledge::{AnnotateTranslationsOptions, annotate_translations};
use stringer_workspace_api::{ExportTranslationsOptions, export_translations};
use stringer_workspace_core::GlobalConfigSource;

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn annotate_large_exact_memory_workspace_auto_fills_and_preserves_order() {
    let root = TempRoot::new("annotate-large-exact-memory");
    let source_root = root.path().join("source");
    let mut source_rows = String::new();
    let mut memory_rows = String::new();
    for index in 0..1000 {
        source_rows.push_str(&format!("$Key{index:04}\tPerf Source {index:04}\n"));
        memory_rows.push_str(&format!(
            "{{\"id\":\"tm:{index:04}\",\"source\":\"Perf Source {index:04}\",\"target\":\"目标 {index:04}\",\"source_locale\":\"en\",\"target_locale\":\"zh-Hans\",\"quality\":\"confirmed\"}}\n"
        ));
    }
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        &source_rows,
    );
    let translations = root.path().join("translations");
    write_text(
        &translations.join("knowledge/memory/workspace.jsonl"),
        &memory_rows,
    );
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: false,
    })
    .await
    .unwrap();

    let summary = annotate_translations(AnnotateTranslationsOptions {
        workspace: utf8(&translations),
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
        skip_memory_fill: false,
    })
    .unwrap();

    assert_eq!(summary.entries, 1000);
    assert_eq!(summary.auto_filled, 1000);
    assert!(summary.index_used);
    let rows = entry_rows(&translations, "scaleform", None);
    assert_eq!(rows.len(), 1000);
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row["source"], format!("Perf Source {index:04}"));
        assert_eq!(row["translation"], format!("目标 {index:04}"));
        assert_eq!(row["translation_meta"]["origin"], "memory");
    }
}
