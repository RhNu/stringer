use stringer_workspace_api::{
    ApplyBatchPatchEntry, ApplyBatchPatchOptions, ClaimBatchOptions, CountBatchOptions,
    ExportTranslationsOptions, InspectWorkspaceBatchOptions, ReleaseBatchOptions,
    apply_batch_patch, claim_batch, count_batch, export_translations, inspect_workspace_batch,
    release_batch,
};

#[allow(dead_code)]
mod support;

use support::*;

#[tokio::test]
async fn batch_count_claim_apply_and_release_manage_claimed_entries() {
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
    assert_eq!(claim.scope.file.as_deref(), Some(file));
    assert!(
        translations
            .join("batches")
            .join(format!("{batch_id}.json"))
            .exists()
    );
    let page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 2,
    })
    .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].source, "Iron Sword");
    assert_eq!(page.entries[1].translation.as_deref(), Some("钢剑"));
    assert_eq!(
        page.entries[1]
            .translation_meta
            .as_ref()
            .and_then(|meta| meta.origin.as_deref()),
        Some("memory")
    );
    assert_eq!(page.entries[1].diagnostics.len(), 1);

    let count = count_batch(CountBatchOptions {
        workspace: utf8(&translations),
        file: Some(file.to_string()),
    })
    .unwrap();
    assert_eq!(count.claimed, 2);

    let apply = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        entries: vec![ApplyBatchPatchEntry {
            id: "scaleform:Interface/Translations/MyMod_English.txt:$Title".to_string(),
            translation: Some("熟铁剑".to_string()),
            skip: false,
            skip_reason: None,
        }],
    })
    .unwrap();
    assert_eq!(apply.applied_entries, 1);
    assert_eq!(apply.remaining_entries, 1);
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
async fn batch_apply_rejects_entries_not_claimed_by_batch() {
    let root = TempRoot::new("batch-apply-rejects-unclaimed");
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

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: None,
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    let claimed_id = page.entries[0].id.as_str();
    let unclaimed_id = if claimed_id.ends_with("$Title") {
        "scaleform:Interface/Translations/MyMod_English.txt:$Desc"
    } else {
        "scaleform:Interface/Translations/MyMod_English.txt:$Title"
    };
    let error = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(&translations),
        batch_id,
        entries: vec![ApplyBatchPatchEntry {
            id: unclaimed_id.to_string(),
            translation: Some("钢剑".to_string()),
            skip: false,
            skip_reason: None,
        }],
    })
    .unwrap_err();

    assert!(error.to_string().contains("is not claimed by batch"));
}

#[tokio::test]
async fn batch_apply_rejects_duplicate_ids_and_missing_translation() {
    let root = TempRoot::new("batch-apply-input-errors");
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
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&translations),
        file: None,
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    let id = page.entries[0].id.clone();

    let duplicate = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(&translations),
        batch_id: batch_id.clone(),
        entries: vec![
            ApplyBatchPatchEntry {
                id: id.clone(),
                translation: Some("铁剑".to_string()),
                skip: false,
                skip_reason: None,
            },
            ApplyBatchPatchEntry {
                id: id.clone(),
                translation: Some("熟铁剑".to_string()),
                skip: false,
                skip_reason: None,
            },
        ],
    })
    .unwrap_err();
    assert!(duplicate.to_string().contains("duplicate batch patch id"));

    let missing = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(&translations),
        batch_id,
        entries: vec![ApplyBatchPatchEntry {
            id,
            translation: None,
            skip: false,
            skip_reason: None,
        }],
    })
    .unwrap_err();
    assert!(missing.to_string().contains("missing translation"));
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
