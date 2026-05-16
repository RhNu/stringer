use stringer_workspace_api::{
    ClaimBatchOptions, ExportTranslationsOptions, InspectDiagnosticSeverity, InspectEntryStatus,
    InspectWorkspaceBatchOptions, InspectWorkspaceDiagnosticsOptions,
    InspectWorkspaceEntriesOptions, InspectWorkspaceEntryOptions, InspectWorkspaceFilesOptions,
    claim_batch, export_translations, inspect_workspace_batch, inspect_workspace_diagnostics,
    inspect_workspace_entries, inspect_workspace_entry, inspect_workspace_files,
};

#[allow(dead_code)]
mod support;

use support::*;

const ENTRY_FILE: &str = "entries/scaleform/Interface/Translations/MyMod_English.txt.jsonl";

#[tokio::test]
async fn inspect_files_returns_manifest_files_without_reading_raw_paths() {
    let fixture = fixture_workspace("inspect-files").await;
    write_entry_rows(&fixture.translations, "scaleform", "{not jsonl}\n");

    let inspected = inspect_workspace_files(InspectWorkspaceFilesOptions {
        workspace: utf8(&fixture.translations),
    })
    .unwrap();

    assert_eq!(inspected.files.len(), 1);
    assert_eq!(inspected.files[0].path, ENTRY_FILE);
    assert_eq!(inspected.files[0].kind, "scaleform");
    assert_eq!(
        inspected.files[0].asset_path,
        "Interface/Translations/MyMod_English.txt"
    );
}

#[tokio::test]
async fn inspect_entries_file_filter_does_not_parse_unselected_entry_files() {
    let fixture = fixture_workspace("inspect-filtered-file").await;
    add_manifest_file(
        &fixture.translations,
        "entries/scaleform/Other.jsonl",
        "scaleform",
        "Other",
    );
    write_text(
        &fixture.translations.join("entries/scaleform/Other.jsonl"),
        "{not jsonl}\n",
    );

    let entries = inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: utf8(&fixture.translations),
        file: Some(ENTRY_FILE.to_string()),
        status: InspectEntryStatus::All,
        limit: 10,
        offset: 0,
    })
    .unwrap();

    assert_eq!(entries.total, 4);
    assert!(entries.entries.iter().all(|entry| entry.file == ENTRY_FILE));
}

#[tokio::test]
async fn inspect_files_rejects_manifest_entry_paths_that_escape_workspace() {
    let fixture = fixture_workspace("inspect-files-invalid-path").await;
    add_manifest_file(
        &fixture.translations,
        "../outside.jsonl",
        "scaleform",
        "Outside",
    );

    let error = inspect_workspace_files(InspectWorkspaceFilesOptions {
        workspace: utf8(&fixture.translations),
    })
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid translation package path")
    );
}

#[tokio::test]
async fn inspect_entries_filters_empty_memory_translated_claimed_and_diagnostic_rows() {
    let fixture = fixture_workspace("inspect-entry-filters").await;
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&fixture.translations),
        file: Some(ENTRY_FILE.to_string()),
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let all = inspect_entries(&fixture.translations, InspectEntryStatus::All);
    assert_eq!(all.total, 4);
    assert_eq!(all.entries.len(), 4);
    assert_eq!(all.entries[0].file, ENTRY_FILE);
    assert_eq!(
        all.entries[0].claimed_by.as_deref(),
        Some(batch_id.as_str())
    );

    let empty = inspect_entries(&fixture.translations, InspectEntryStatus::Empty);
    assert_eq!(empty.total, 2);
    assert_eq!(empty.entries[0].source, "Iron Sword");

    let memory = inspect_entries(&fixture.translations, InspectEntryStatus::Memory);
    assert_eq!(memory.total, 1);
    assert_eq!(memory.entries[0].source, "Steel Sword");

    let entry_path = fixture.translations.join(ENTRY_FILE);
    write_text(
        &entry_path,
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\",\"translation_meta\":{\"origin\":\"memory\"}}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Done\",\"source\":\"Done\",\"translation\":\"完成\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Warn\",\"source\":\"Needs Review\",\"diagnostics\":[{\"severity\":\"info\",\"code\":\"review.note\",\"message\":\"inspect\"}]}\n",
        ),
    );
    let memory = inspect_entries(&fixture.translations, InspectEntryStatus::Memory);
    assert_eq!(memory.total, 1);
    assert_eq!(memory.entries[0].source, "Steel Sword");

    let translated = inspect_entries(&fixture.translations, InspectEntryStatus::Translated);
    assert_eq!(translated.total, 1);
    assert_eq!(translated.entries[0].source, "Done");

    let claimed = inspect_entries(&fixture.translations, InspectEntryStatus::Claimed);
    assert_eq!(claimed.total, 1);
    assert_eq!(
        claimed.entries[0].claimed_by.as_deref(),
        Some(batch_id.as_str())
    );

    let diagnostic = inspect_entries(&fixture.translations, InspectEntryStatus::Diagnostic);
    assert_eq!(diagnostic.total, 2);
    assert_eq!(diagnostic.entries[0].diagnostics.len(), 1);
}

#[tokio::test]
async fn inspect_entry_returns_single_record_by_id() {
    let fixture = fixture_workspace("inspect-entry").await;

    let entry = inspect_workspace_entry(InspectWorkspaceEntryOptions {
        workspace: utf8(&fixture.translations),
        id: "scaleform:Interface/Translations/MyMod_English.txt:$Desc".to_string(),
    })
    .unwrap();

    assert_eq!(entry.file, ENTRY_FILE);
    assert_eq!(entry.source, "Steel Sword");
    assert_eq!(entry.translation.as_deref(), Some("钢剑"));
    assert_eq!(entry.diagnostics[0].code(), "memory.conflict");
}

#[tokio::test]
async fn inspect_batch_returns_claimed_remaining_entries_without_applying() {
    let fixture = fixture_workspace("inspect-batch").await;
    let claim = claim_batch(ClaimBatchOptions {
        workspace: utf8(&fixture.translations),
        file: Some(ENTRY_FILE.to_string()),
        limit: 2,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");

    let batch = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(&fixture.translations),
        batch_id: batch_id.clone(),
        offset: 0,
        limit: 1,
    })
    .unwrap();

    assert_eq!(batch.batch_id, batch_id);
    assert_eq!(batch.total, 2);
    assert_eq!(batch.offset, 0);
    assert_eq!(batch.limit, 1);
    assert_eq!(batch.next_offset, None);
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(
        batch.entries[0].claimed_by.as_deref(),
        Some(batch.batch_id.as_str())
    );
    let rows = entry_rows(&fixture.translations, "scaleform", None);
    let title = rows
        .iter()
        .find(|row| row["source"] == "Iron Sword")
        .unwrap();
    assert!(title.get("translation").is_none());
    assert!(title.get("translation_meta").is_none());
}

#[tokio::test]
async fn inspect_diagnostics_expands_entry_diagnostics_for_review() {
    let fixture = fixture_workspace("inspect-diagnostics").await;

    let diagnostics = inspect_workspace_diagnostics(InspectWorkspaceDiagnosticsOptions {
        workspace: utf8(&fixture.translations),
        file: Some(ENTRY_FILE.to_string()),
        severity: InspectDiagnosticSeverity::Warning,
        limit: 10,
        offset: 0,
    })
    .unwrap();

    assert_eq!(diagnostics.total, 1);
    assert_eq!(
        diagnostics.diagnostics[0].entry_id,
        "scaleform:Interface/Translations/MyMod_English.txt:$Desc"
    );
    assert_eq!(diagnostics.diagnostics[0].file, ENTRY_FILE);
    assert_eq!(diagnostics.diagnostics[0].source, "Steel Sword");
    assert_eq!(
        diagnostics.diagnostics[0].translation.as_deref(),
        Some("钢剑")
    );
    assert_eq!(
        diagnostics.diagnostics[0].diagnostic.code(),
        "memory.conflict"
    );
}

#[tokio::test]
async fn inspect_rejects_file_not_listed_in_workspace_manifest() {
    let fixture = fixture_workspace("inspect-invalid-file").await;

    let error = inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: utf8(&fixture.translations),
        file: Some("entries/scaleform/outside.jsonl".to_string()),
        status: InspectEntryStatus::All,
        limit: 10,
        offset: 0,
    })
    .unwrap_err();

    assert!(error.to_string().contains("entry file is not listed"));
}

fn inspect_entries(
    translations: &std::path::Path,
    status: InspectEntryStatus,
) -> stringer_workspace_api::WorkspaceInspectEntries {
    inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: utf8(translations),
        file: Some(ENTRY_FILE.to_string()),
        status,
        limit: 10,
        offset: 0,
    })
    .unwrap()
}

struct InspectFixture {
    _root: TempRoot,
    translations: std::path::PathBuf,
}

async fn fixture_workspace(label: &str) -> InspectFixture {
    let root = TempRoot::new(label);
    let source_root = root.path().join("source");
    write_text(
        &source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSteel Sword\n$Done\tDone\n$Warn\tNeeds Review\n",
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
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Done\",\"source\":\"Done\",\"translation\":\"完成\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Warn\",\"source\":\"Needs Review\",\"diagnostics\":[{\"severity\":\"info\",\"code\":\"review.note\",\"message\":\"inspect\"}]}\n",
        ),
    );
    InspectFixture {
        _root: root,
        translations,
    }
}

fn add_manifest_file(package: &std::path::Path, path: &str, kind: &str, asset_path: &str) {
    let manifest_path = package.join("workspace.json");
    let mut manifest = json_file(&manifest_path);
    manifest["files"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "path": path,
            "kind": kind,
            "asset_path": asset_path
        }));
    write_text(
        &manifest_path,
        &serde_json::to_string_pretty(&manifest).unwrap(),
    );
}
