use stringer_workspace_ops::{
    BatchSubmitAction, BatchSubmitEntry, BatchSubmitOptions, ClaimBatchOptions, CountBatchOptions,
    ReadBatchOptions, ReleaseBatchOptions, claim_batch, count_batch, read_batch, release_batch,
    submit_batch,
};

mod support;

use support::*;

#[test]
fn batch_count_claim_submit_and_release_manage_claimed_entries() {
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
    assert_eq!(count.skipped, 0);
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
    assert_eq!(claim.revision, Some(1));
    assert_eq!(claim.scope.file.as_deref(), Some(ENTRY_FILE));

    let page = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    assert_eq!(page.total_entries, 2);
    assert_eq!(page.offset, 0);
    assert_eq!(page.limit, 1);
    assert_eq!(page.next_offset, Some(1));
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].key, "e001");
    assert_eq!(page.entries[0].source, "Iron Sword");

    let count = count_batch(CountBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
    })
    .unwrap();
    assert_eq!(count.claimed, 2);

    let submitted = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
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
    assert_eq!(submitted.revision, 2);
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
fn batch_submit_skip_marks_entry_done_without_translation() {
    let fixture = workspace_with_rows("batch-skip", rows());

    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let page = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();
    let key = page.entries[0].key.clone();
    let source = page.entries[0].source.clone();

    let submitted = submit_batch(BatchSubmitOptions {
        workspace: utf8(fixture.workspace()),
        batch_id,
        revision: page.revision,
        entries: vec![BatchSubmitEntry {
            key,
            action: BatchSubmitAction::Skip,
            translation: None,
            skip_reason: Some("not_translatable".to_string()),
        }],
    })
    .unwrap();
    assert_eq!(submitted.applied_entries, 1);
    assert_eq!(submitted.remaining_entries, 0);

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    let skipped = rows.iter().find(|row| row["source"] == source).unwrap();
    assert!(skipped.get("translation").is_none());
    assert_eq!(skipped["translation_meta"]["origin"], "skipped");
    assert_eq!(
        skipped["translation_meta"]["skip_reason"],
        "not_translatable"
    );

    let count = count_batch(CountBatchOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
    })
    .unwrap();
    assert_eq!(count.empty, 1);
    assert_eq!(count.skipped, 1);
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

    let first_page = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: first_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();
    let second_page = read_batch(ReadBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: second_id,
        offset: 0,
        limit: 10,
    })
    .unwrap();

    assert_ne!(first_page.entries[0].source, second_page.entries[0].source);
}
