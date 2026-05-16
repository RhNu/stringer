use stringer_workspace_ops::{
    InspectDiagnosticSeverity, InspectEntryStatus, InspectWorkspaceBatchOptions,
    InspectWorkspaceDiagnosticsOptions, InspectWorkspaceEntriesOptions,
    InspectWorkspaceFilesOptions, inspect_workspace_batch, inspect_workspace_diagnostics,
    inspect_workspace_entries, inspect_workspace_files,
};

mod support;

use support::*;

#[test]
fn inspect_files_returns_manifest_files_without_parsing_entries() {
    let fixture = workspace_with_rows("inspect-files", "{not json}\n");

    let inspected = inspect_workspace_files(InspectWorkspaceFilesOptions {
        workspace: utf8(fixture.workspace()),
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

#[test]
fn inspect_entries_filters_statuses_and_claims() {
    let fixture = workspace_with_rows("inspect-entries", rows());
    write_batch(fixture.workspace(), "b-test", &["scaleform:MyMod:$Title"]);

    let all = inspect_entries(fixture.workspace(), InspectEntryStatus::All);
    assert_eq!(all.total, 4);
    assert_eq!(all.entries[0].claimed_by.as_deref(), Some("b-test"));

    let empty = inspect_entries(fixture.workspace(), InspectEntryStatus::Empty);
    assert_eq!(empty.total, 2);
    assert_eq!(empty.entries[0].source, "Iron Sword");

    let memory = inspect_entries(fixture.workspace(), InspectEntryStatus::Memory);
    assert_eq!(memory.total, 1);
    assert_eq!(memory.entries[0].source, "Steel Sword");

    let translated = inspect_entries(fixture.workspace(), InspectEntryStatus::Translated);
    assert_eq!(translated.total, 1);
    assert_eq!(translated.entries[0].source, "Done");

    let claimed = inspect_entries(fixture.workspace(), InspectEntryStatus::Claimed);
    assert_eq!(claimed.total, 1);
    assert_eq!(claimed.entries[0].source, "Iron Sword");

    let diagnostic = inspect_entries(fixture.workspace(), InspectEntryStatus::Diagnostic);
    assert_eq!(diagnostic.total, 2);
}

#[test]
fn inspect_batch_and_diagnostics_read_hand_written_batch_fixtures() {
    let fixture = workspace_with_rows("inspect-batch", rows());
    write_batch(
        fixture.workspace(),
        "b-review",
        &["scaleform:MyMod:$Title", "scaleform:MyMod:$Desc"],
    );

    let batch = inspect_workspace_batch(InspectWorkspaceBatchOptions {
        workspace: utf8(fixture.workspace()),
        batch_id: "b-review".to_string(),
    })
    .unwrap();
    assert_eq!(batch.batch_id, "b-review");
    assert_eq!(batch.entries.len(), 2);
    assert_eq!(batch.entries[0].claimed_by.as_deref(), Some("b-review"));

    let diagnostics = inspect_workspace_diagnostics(InspectWorkspaceDiagnosticsOptions {
        workspace: utf8(fixture.workspace()),
        file: Some(ENTRY_FILE.to_string()),
        severity: InspectDiagnosticSeverity::Warning,
        limit: 10,
        offset: 0,
    })
    .unwrap();
    assert_eq!(diagnostics.total, 1);
    assert_eq!(diagnostics.diagnostics[0].entry_id, "scaleform:MyMod:$Desc");
    assert_eq!(
        diagnostics.diagnostics[0].diagnostic.code(),
        "memory.conflict"
    );
}

fn inspect_entries(
    workspace: &std::path::Path,
    status: InspectEntryStatus,
) -> stringer_workspace_ops::WorkspaceInspectEntries {
    inspect_workspace_entries(InspectWorkspaceEntriesOptions {
        workspace: utf8(workspace),
        file: Some(ENTRY_FILE.to_string()),
        status,
        limit: 10,
        offset: 0,
    })
    .unwrap()
}
