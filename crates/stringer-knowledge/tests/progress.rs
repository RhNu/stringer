use stringer_knowledge::{
    KnowledgeOperation, KnowledgeProgressPhase, annotate_translations_with_progress,
};
use stringer_workspace_api::{ExportTranslationsOptions, export_translations};
use stringer_workspace_core::GlobalConfigSource;

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn annotate_translations_reports_progress_for_each_entry() {
    let root = TempRoot::new("annotate-progress");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSteel Sword\n",
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

    let mut events = Vec::new();
    let summary = annotate_translations_with_progress(
        stringer_knowledge::AnnotateTranslationsOptions {
            workspace: utf8(&translations),
            global_config_source: GlobalConfigSource::FixedKnowledgeRoot(None),
            skip_memory_fill: true,
        },
        |event| {
            events.push(event);
        },
    )
    .unwrap();

    assert_eq!(summary.entries, 2);
    assert_eq!(events[0].operation, KnowledgeOperation::Annotate);
    assert_eq!(events[0].phase, KnowledgeProgressPhase::Started);
    assert_eq!(events[0].processed, 0);
    assert_eq!(events[0].total, Some(2));
    let advanced = events
        .iter()
        .filter(|event| event.phase == KnowledgeProgressPhase::Advanced)
        .collect::<Vec<_>>();
    assert_eq!(advanced.len(), 2);
    assert_eq!(advanced[0].processed, 1);
    assert_eq!(advanced[1].processed, 2);
    assert_eq!(
        events.last().unwrap().phase,
        KnowledgeProgressPhase::Finished
    );
    assert_eq!(events.last().unwrap().processed, 2);
}
