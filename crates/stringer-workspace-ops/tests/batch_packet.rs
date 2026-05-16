use std::fs;

use serde_json::Value;
use stringer_workspace_core::WorkspaceCoreError;
use stringer_workspace_ops::{
    BatchExportFormat, BatchExportOptions, BatchSubmitAction, BatchSubmitEntry, BatchSubmitOptions,
    BatchSubmitStatus, ClaimBatchOptions, CountBatchOptions, ReadBatchDetailOptions,
    ReadBatchOptions, WorkspaceOpsError, claim_batch, count_batch, export_batch_submission,
    read_batch, read_batch_detail, submit_batch,
};

mod support;

use support::*;

#[test]
fn batch_claim_read_detail_and_submit_use_stable_keys_and_revisions() {
    let fixture = workspace_with_rows("packet-flow", rows());

    let count = count_batch(CountBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
    })
    .unwrap();
    assert_eq!(count.claimable, 3);

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 3,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    assert_eq!(claim.revision, Some(1));
    assert_eq!(claim.claimed_entries, 3);
    assert_eq!(claim.remaining_claimable, 0);

    let page = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 2,
    })
    .unwrap();
    assert_eq!(page.revision, 1);
    assert_eq!(page.total_entries, 3);
    assert_eq!(page.open_entries, 3);
    assert_eq!(page.next_offset, Some(2));
    assert_eq!(page.entries[0].key, "e001");
    assert_eq!(page.entries[0].source, "Iron Sword");
    assert_eq!(page.entries[0].current_translation, None);
    assert_eq!(page.entries[0].context_label, "scaleform $Title");
    assert_eq!(page.entries[0].hint_count, 0);
    assert_eq!(page.entries[0].diagnostic_count, 0);
    assert_eq!(page.entries[1].key, "e002");
    assert_eq!(page.entries[1].current_translation.as_deref(), Some("钢剑"));
    assert_eq!(page.entries[1].origin.as_deref(), Some("memory"));
    assert_eq!(page.entries[1].diagnostic_codes, vec!["memory.conflict"]);

    let detail = read_batch_detail(ReadBatchDetailOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        keys: vec!["e002".to_string()],
    })
    .unwrap();
    assert_eq!(detail.revision, 1);
    assert_eq!(detail.entries.len(), 1);
    assert_eq!(detail.entries[0].key, "e002");
    assert_eq!(detail.entries[0].context["key"], "$Desc");
    assert_eq!(detail.entries[0].diagnostics.len(), 1);

    let empty_detail_error = read_batch_detail(ReadBatchDetailOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        keys: Vec::new(),
    })
    .unwrap_err();
    assert!(matches!(
        empty_detail_error,
        WorkspaceOpsError::BatchDetailKeysRequired { .. }
    ));

    let summary = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        revision: 1,
        entries: vec![
            BatchSubmitEntry {
                key: "e001".to_string(),
                action: BatchSubmitAction::Translate,
                translation: Some("熟铁剑".to_string()),
                skip_reason: None,
            },
            BatchSubmitEntry {
                key: "e002".to_string(),
                action: BatchSubmitAction::Skip,
                translation: None,
                skip_reason: Some("not_translatable".to_string()),
            },
            BatchSubmitEntry {
                key: "e003".to_string(),
                action: BatchSubmitAction::Pending,
                translation: None,
                skip_reason: None,
            },
        ],
    })
    .unwrap();
    assert_eq!(summary.revision, 2);
    assert_eq!(summary.applied_entries, 2);
    assert_eq!(summary.ignored_entries, 1);
    assert_eq!(summary.rejected_entries, 0);
    assert_eq!(summary.remaining_entries, 1);
    assert_eq!(summary.next_read_offset, 0);
    assert_eq!(summary.results[0].status, BatchSubmitStatus::Applied);
    assert_eq!(summary.results[2].status, BatchSubmitStatus::Ignored);

    let remaining = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();
    assert_eq!(remaining.revision, 2);
    assert_eq!(remaining.entries.len(), 1);
    assert_eq!(remaining.entries[0].key, "e003");

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "熟铁剑");
    assert_eq!(rows[0]["translation_meta"]["origin"], "agent");
    assert!(rows[1].get("translation").is_none());
    assert_eq!(rows[1]["translation_meta"]["origin"], "skipped");
}

#[test]
fn batch_submit_rejects_stale_revision_before_submitting_entries() {
    let fixture = workspace_with_rows("packet-stale-revision", rows());
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let error = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        revision: 0,
        entries: vec![BatchSubmitEntry {
            key: "e001".to_string(),
            action: BatchSubmitAction::Translate,
            translation: Some("熟铁剑".to_string()),
            skip_reason: None,
        }],
    })
    .unwrap_err();

    assert!(matches!(
        error,
        WorkspaceOpsError::BatchRevisionConflict { current: 1, .. }
    ));
    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert!(rows[0].get("translation").is_none());
}

#[test]
fn batch_submit_rejects_empty_translation_and_pending_payload() {
    let fixture = workspace_with_rows("packet-empty-translation", rows());
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let summary = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        revision: 1,
        entries: vec![
            BatchSubmitEntry {
                key: "e001".to_string(),
                action: BatchSubmitAction::Translate,
                translation: Some("  ".to_string()),
                skip_reason: None,
            },
            BatchSubmitEntry {
                key: "e002".to_string(),
                action: BatchSubmitAction::Pending,
                translation: Some("钢剑".to_string()),
                skip_reason: None,
            },
        ],
    })
    .unwrap();

    assert_eq!(summary.applied_entries, 0);
    assert_eq!(summary.ignored_entries, 0);
    assert_eq!(summary.rejected_entries, 2);
    assert_eq!(summary.revision, 1);
    assert_eq!(summary.remaining_entries, 2);
    assert_eq!(
        summary.results[0].message.as_deref(),
        Some("translate action requires non-empty translation")
    );
    assert_eq!(
        summary.results[1].message.as_deref(),
        Some("pending action must not include translation or skip_reason")
    );

    let remaining = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();
    assert_eq!(remaining.revision, 1);
    assert_eq!(remaining.entries.len(), 2);
}

#[test]
fn batch_submit_rejects_mismatched_loaded_batch_id_before_writing_rows() {
    let fixture = workspace_with_rows("packet-mismatched-batch-id", rows());
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let batch_path = fixture
        .workspace()
        .join("batches")
        .join(format!("{batch_id}.json"));
    let mut batch_json: Value =
        serde_json::from_str(&fs::read_to_string(&batch_path).unwrap()).unwrap();
    batch_json["batch_id"] = Value::String("../escape".to_string());
    write_text(
        &batch_path,
        &serde_json::to_string_pretty(&batch_json).unwrap(),
    );

    let error = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        revision: 1,
        entries: vec![BatchSubmitEntry {
            key: "e001".to_string(),
            action: BatchSubmitAction::Translate,
            translation: Some("熟铁剑".to_string()),
            skip_reason: None,
        }],
    })
    .unwrap_err();

    assert!(matches!(
        error,
        WorkspaceOpsError::Core(WorkspaceCoreError::InvalidTranslationPackagePath { .. })
    ));
    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert!(rows[0].get("translation").is_none());
}

#[test]
fn batch_submit_rejects_bad_entries_without_blocking_valid_entries() {
    let fixture = workspace_with_rows("packet-partial-reject", rows());
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let summary = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        revision: 1,
        entries: vec![
            BatchSubmitEntry {
                key: "e001".to_string(),
                action: BatchSubmitAction::Translate,
                translation: Some("熟铁剑".to_string()),
                skip_reason: None,
            },
            BatchSubmitEntry {
                key: "e001".to_string(),
                action: BatchSubmitAction::Translate,
                translation: Some("重复".to_string()),
                skip_reason: None,
            },
            BatchSubmitEntry {
                key: "e999".to_string(),
                action: BatchSubmitAction::Translate,
                translation: Some("不存在".to_string()),
                skip_reason: None,
            },
            BatchSubmitEntry {
                key: "e002".to_string(),
                action: BatchSubmitAction::Translate,
                translation: None,
                skip_reason: None,
            },
        ],
    })
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.rejected_entries, 3);
    assert_eq!(summary.remaining_entries, 1);
    assert_eq!(summary.results[0].status, BatchSubmitStatus::Applied);
    assert_eq!(summary.results[1].status, BatchSubmitStatus::Rejected);
    assert_eq!(
        summary.results[1].message.as_deref(),
        Some("duplicate key in submit entries")
    );
    assert_eq!(summary.results[2].status, BatchSubmitStatus::Rejected);
    assert_eq!(summary.results[3].status, BatchSubmitStatus::Rejected);
}

#[test]
fn batch_export_json_and_csv_submission_files_can_be_submitted() {
    let fixture = workspace_with_rows("packet-export", rows());
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let exported = export_batch_submission(BatchExportOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        out: None,
        format: BatchExportFormat::Json,
    })
    .unwrap();
    assert_eq!(exported.entries, 2);
    assert!(
        exported
            .path
            .ends_with(&format!("batch-work/{batch_id}/patch.json"))
    );
    let mut patch: Value =
        serde_json::from_str(&fs::read_to_string(&exported.path).unwrap()).unwrap();
    patch["entries"][0]["action"] = Value::String("translate".to_string());
    patch["entries"][0]["translation"] = Value::String("熟铁剑".to_string());
    patch["entries"][1]["action"] = Value::String("pending".to_string());
    write_text(
        std::path::Path::new(exported.path.as_str()),
        &serde_json::to_string_pretty(&patch).unwrap(),
    );

    let summary = submit_batch(
        BatchSubmitOptions::from_json_file(utf8(fixture.workspace()), exported.path.clone())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(summary.applied_entries, 1);

    let exported_csv = export_batch_submission(BatchExportOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        out: None,
        format: BatchExportFormat::Csv,
    })
    .unwrap();
    assert_eq!(exported_csv.entries, 1);
    let csv_text = fs::read_to_string(&exported_csv.path).unwrap();
    assert!(csv_text.starts_with("# stringer batch_id="));
    assert!(csv_text.contains(
        "key,source,current_translation,context_label,diagnostic_codes,action,translation,skip_reason"
    ));
}
