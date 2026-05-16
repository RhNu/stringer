use stringer_workspace_ops::{
    ApplyBatchPatchEntry, ApplyBatchPatchOptions, ClaimBatchOptions, CountBatchOptions,
    InspectWorkspaceBatchOptions, ReleaseBatchOptions, WorkspaceOpsError, apply_batch_patch,
    claim_batch, count_batch, inspect_workspace_batch, release_batch,
};

mod support;

use support::*;

#[test]
fn batch_count_claim_apply_and_release_manage_claimed_entries() {
    let fixture = workspace_with_rows("batch-flow", rows());

    let count = count_batch(CountBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
    })
    .unwrap();
    assert_eq!(count.total, 4);
    assert_eq!(count.empty, 2);
    assert_eq!(count.memory_prefilled, 1);
    assert_eq!(count.translated, 1);
    assert_eq!(count.claimed, 0);
    assert_eq!(count.diagnostics, 2);

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    assert_eq!(claim.claimed_entries, 2);
    assert_eq!(claim.scope.file.as_deref(), Some(ENTRY_FILE));

    let batch_page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    assert_eq!(batch_page.total, 2);
    assert_eq!(batch_page.offset, 0);
    assert_eq!(batch_page.limit, 1);
    assert_eq!(batch_page.next_offset, None);
    assert_eq!(batch_page.entries.len(), 1);
    assert_eq!(batch_page.entries[0].source, "Iron Sword");

    let count = count_batch(CountBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
    })
    .unwrap();
    assert_eq!(count.claimed, 2);

    let apply = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        entries: vec![ApplyBatchPatchEntry {
            id: "scaleform:MyMod:$Title".to_string(),
            translation: Some("熟铁剑".to_string()),
        }],
    })
    .unwrap();
    assert_eq!(apply.applied_entries, 1);
    assert_eq!(apply.remaining_entries, 1);
    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "熟铁剑");
    assert_eq!(rows[0]["translation_meta"]["origin"], "agent");
    assert!(rows[0]["translation_meta"]["updated_at_unix_ms"].is_number());

    let released = release_batch(ReleaseBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
    })
    .unwrap();
    assert_eq!(released.released_entries, 1);
    assert!(
        !fixture
            .workspace()
            .join("batches")
            .join(format!("{batch_id}.json"))
            .exists()
    );
}

#[test]
fn batch_inspect_omits_next_offset_because_apply_mutates_remaining_entries() {
    let fixture = workspace_with_rows("batch-mutable-page", rows());

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let first_page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    assert_eq!(first_page.entries.len(), 1);
    assert_eq!(first_page.next_offset, None);

    apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        entries: vec![ApplyBatchPatchEntry {
            id: first_page.entries[0].id.clone(),
            translation: Some("熟铁剑".to_string()),
        }],
    })
    .unwrap();

    let remaining_page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        offset: 0,
        limit: 1,
    })
    .unwrap();
    assert_eq!(remaining_page.total, 1);
    assert_eq!(remaining_page.entries.len(), 1);
}

#[test]
fn consecutive_claims_create_distinct_non_overlapping_batches() {
    let fixture = workspace_with_rows("batch-unique", rows());

    let first = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let first_id = first.batch_id.expect("first batch id");
    let second = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let second_id = second.batch_id.expect("second batch id");

    assert_ne!(first_id, second_id);
    assert_eq!(first.claimed_entries, 1);
    assert_eq!(second.claimed_entries, 1);

    let first_page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: first_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();
    let second_page = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: second_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();

    assert_ne!(first_page.entries[0].id, second_page.entries[0].id);
}

#[test]
fn batch_apply_rejects_duplicate_missing_unclaimed_and_unknown_entries() {
    let fixture = workspace_with_rows("batch-errors", rows());
    write_batch(
        fixture.workspace(),
        "b-test",
        &["scaleform:MyMod:$Title", "scaleform:MyMod:$Missing"],
    );

    let duplicate = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: "b-test".to_string(),
        entries: vec![
            ApplyBatchPatchEntry {
                id: "scaleform:MyMod:$Title".to_string(),
                translation: Some("铁剑".to_string()),
            },
            ApplyBatchPatchEntry {
                id: "scaleform:MyMod:$Title".to_string(),
                translation: Some("熟铁剑".to_string()),
            },
        ],
    })
    .unwrap_err();
    assert!(matches!(
        duplicate,
        WorkspaceOpsError::DuplicateBatchPatchId { .. }
    ));

    let missing_translation = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: "b-test".to_string(),
        entries: vec![ApplyBatchPatchEntry {
            id: "scaleform:MyMod:$Title".to_string(),
            translation: None,
        }],
    })
    .unwrap_err();
    assert!(matches!(
        missing_translation,
        WorkspaceOpsError::MissingBatchPatchTranslation { .. }
    ));

    let unclaimed = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: "b-test".to_string(),
        entries: vec![ApplyBatchPatchEntry {
            id: "scaleform:MyMod:$Desc".to_string(),
            translation: Some("钢剑".to_string()),
        }],
    })
    .unwrap_err();
    assert!(matches!(
        unclaimed,
        WorkspaceOpsError::BatchEntryNotClaimed { .. }
    ));

    let unknown = apply_batch_patch(ApplyBatchPatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: "b-test".to_string(),
        entries: vec![ApplyBatchPatchEntry {
            id: "scaleform:MyMod:$Missing".to_string(),
            translation: Some("不存在".to_string()),
        }],
    })
    .unwrap_err();
    assert!(matches!(
        unknown,
        WorkspaceOpsError::UnknownTranslationId { .. }
    ));
}
