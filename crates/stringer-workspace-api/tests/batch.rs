use stringer_workspace_api::{
    BatchSubmitAction, BatchSubmitEntry, BatchSubmitOptions, ClaimBatchOptions, CountBatchOptions,
    ExportTranslationsOptions, ReadBatchOptions, ReleaseBatchOptions, claim_batch, count_batch,
    export_translations, read_batch, release_batch, submit_batch,
};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn batch_count_claim_submit_and_release_manage_claimed_entries() {
    let root = TempRoot::new("batch-flow");
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSteel Sword\n$Done\tDone\n",
    );
    let translations = root.path().join("translations");
    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: true,
    })
    .await
    .unwrap();
    write_entry_rows(
        &translations,
        "scaleform",
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Done\",\"source\":\"Done\",\"translation\":\"完成\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
        ),
    );
    let file = "entries/scaleform/Interface/Translations/MyMod_English.txt.jsonl";

    let count = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
    })
    .unwrap();
    assert_eq!(count.total, 3);
    assert_eq!(count.empty, 1);
    assert_eq!(count.memory_prefilled, 1);
    assert_eq!(count.translated, 1);
    assert_eq!(count.claimed, 0);
    assert_eq!(count.diagnostics, 1);

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    assert_eq!(claim.claimed_entries, 2);
    assert_eq!(claim.revision, Some(1));
    assert_eq!(claim.scope.file.as_deref(), Some(file));
    assert!(
        translations
            .join("batches")
            .join(format!("{batch_id}.json"))
            .exists()
    );
    let page = read_batch(ReadBatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 2,
    })
    .unwrap();
    assert_eq!(page.total_entries, 2);
    assert_eq!(page.entries[0].source, "Iron Sword");
    assert_eq!(page.entries[1].current_translation.as_deref(), Some("钢剑"));
    assert_eq!(page.entries[1].origin.as_deref(), Some("memory"));
    assert_eq!(page.entries[1].diagnostic_count, 1);

    let count = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
    })
    .unwrap();
    assert_eq!(count.claimed, 2);

    let submitted = submit_batch(BatchSubmitOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        revision: page.revision,
        entries: vec![BatchSubmitEntry {
            key: page.entries[0].key.clone(),
            action: BatchSubmitAction::Translate,
            translation: Some("熟铁剑".to_string()),
            skip_reason: None,
        }],
    })
    .unwrap();
    assert_eq!(submitted.applied_entries, 1);
    assert_eq!(submitted.remaining_entries, 1);
    let rows = entry_rows(&translations, "scaleform", None);
    let title = rows
        .iter()
        .find(|row| row["source"] == "Iron Sword")
        .unwrap();
    assert_eq!(title["translation"], "熟铁剑");
    assert_eq!(title["translation_meta"]["origin"], "agent");
    assert!(title["translation_meta"]["updated_at_unix_ms"].is_number());
    assert!(
        translations
            .join("batches")
            .join(format!("{batch_id}.json"))
            .exists()
    );

    let released = release_batch(ReleaseBatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
    })
    .unwrap();
    assert_eq!(released.released_entries, 1);
    assert!(
        !translations
            .join("batches")
            .join(format!("{batch_id}.json"))
            .exists()
    );
}

#[tokio::test]
async fn force_workspace_open_clears_existing_batch_claims() {
    let root = TempRoot::new("batch-open-clears-claims");
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
        force: true,
    })
    .await
    .unwrap();
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: None,
        limit: 1,
    })
    .unwrap();
    assert!(claim.batch_id.is_some());
    assert!(
        translations
            .join("batches")
            .read_dir()
            .unwrap()
            .next()
            .is_some()
    );

    export_translations(ExportTranslationsOptions {
        source_root: utf8(&source_root),
        workspace: utf8(&translations),
        settings: settings(),
        force: true,
    })
    .await
    .unwrap();

    assert!(translations.join("batches").is_dir());
    assert!(
        translations
            .join("batches")
            .read_dir()
            .unwrap()
            .next()
            .is_none()
    );
    let count = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: None,
    })
    .unwrap();
    assert_eq!(count.claimed, 0);
}

#[tokio::test]
async fn batch_count_rejects_malformed_batch_claim_files() {
    let root = TempRoot::new("batch-malformed-claim");
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
    write_text(
        &translations.join("batches/malformed.json"),
        r#"{"batch_id":"malformed","entry_ids":["scaleform:Interface/Translations/MyMod_English.txt:$Title"]}"#,
    );

    let error = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: None,
    })
    .unwrap_err();

    assert!(error.to_string().contains("failed to process JSON"));
}

#[tokio::test]
async fn batch_file_filters_accept_windows_path_separators() {
    let root = TempRoot::new("batch-windows-file-filter");
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
    let file = "entries\\scaleform\\Interface\\Translations\\MyMod_English.txt.jsonl";

    let count = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
    })
    .unwrap();
    assert_eq!(count.total, 1);

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
        limit: 1,
    })
    .unwrap();
    assert_eq!(claim.claimed_entries, 1);
}

#[tokio::test]
async fn release_rejects_batch_id_paths() {
    let root = TempRoot::new("batch-id-path");
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

    let error = release_batch(ReleaseBatchOptions {
        workspace: utf8(&translations),
        batch_id: "../outside".to_string(),
    })
    .unwrap_err();

    assert!(error.to_string().contains("batch id must be a file name"));
}

#[tokio::test]
async fn workspace_lock_blocks_second_mutating_command() {
    let root = TempRoot::new("workspace-lock");
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
    write_text(
        &translations.join("lock"),
        "{\"pid\":1,\"created_at_unix_ms\":1}\n",
    );

    let error = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: None,
        limit: 1,
    })
    .unwrap_err();

    assert!(error.to_string().contains("workspace is locked"));
    assert!(translations.join("lock").exists());
}
