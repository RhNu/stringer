use std::fs;

use serde_json::Value;
use stringer_app::{
    AdaptFormatInput, AdaptImportRequest, InspectDiagnosticSeverityInput, InspectEntryStatusInput,
    SettingsInput, WorkspaceBatchApplyEntry, WorkspaceBatchApplyRequest,
    WorkspaceBatchClaimRequest, WorkspaceBatchCountRequest, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectEntriesRequest, WorkspaceOpenRequest, adapt_import, workspace_batch_apply,
    workspace_batch_claim, workspace_batch_count, workspace_inspect_diagnostics,
    workspace_inspect_entries, workspace_open,
};

#[tokio::test]
async fn app_workspace_batch_flow_matches_agent_cli_semantics() {
    let root = TempRoot::new("workspace-root");
    let workspace = TempRoot::new("workspace-output");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n");

    let opened = workspace_open(WorkspaceOpenRequest {
        root: path_string(root.path()),
        workspace: path_string(workspace.path()),
        settings: settings(),
    })
    .await
    .unwrap();
    assert_eq!(opened.entries, 1);

    let count = workspace_batch_count(WorkspaceBatchCountRequest {
        workspace: path_string(workspace.path()),
        file: None,
    })
    .unwrap();
    assert_eq!(count.total, 1);
    assert_eq!(count.empty, 1);

    let claim = workspace_batch_claim(WorkspaceBatchClaimRequest {
        workspace: path_string(workspace.path()),
        file: None,
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let entry_id = claim.entries[0].id.clone();
    assert_eq!(claim.entries[0].source, "Iron Sword");

    let summary = workspace_batch_apply(WorkspaceBatchApplyRequest {
        workspace: path_string(workspace.path()),
        batch_id,
        entries: vec![WorkspaceBatchApplyEntry {
            id: entry_id,
            translation: Some("熟铁剑".to_string()),
        }],
    })
    .unwrap();
    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.remaining_entries, 0);
}

#[tokio::test]
async fn app_adapt_import_returns_action_output_and_summary() {
    let temp = TempRoot::new("adapt");
    let input = temp.path().join("source.eet");
    let output = temp.path().join("memory.jsonl");
    fs::write(&input, eet_v1_fixture()).unwrap();

    let imported = adapt_import(AdaptImportRequest {
        format: AdaptFormatInput::Eet,
        input: path_string(&input),
        out: Some(path_string(&output)),
        source_locale: "en".to_string(),
        target_locale: "zh-Hans".to_string(),
        game: Some("skyrim-se".to_string()),
    })
    .await
    .unwrap();

    assert_eq!(imported.action, "wrote");
    assert_eq!(imported.output, path_string(&output));
    assert_eq!(imported.summary.total_entries, 1);
    assert_eq!(imported.summary.written_entries, 1);

    let row: Value = serde_json::from_str(fs::read_to_string(output).unwrap().trim()).unwrap();
    assert_eq!(row["source"], "Iron Sword");
    assert_eq!(row["target"], "铁剑");
}

#[tokio::test]
async fn app_workspace_inspect_entries_and_diagnostics_return_agent_safe_json_values() {
    let root = TempRoot::new("inspect-root");
    let workspace = TempRoot::new("inspect-output");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n$Desc\tSteel Sword\n");

    workspace_open(WorkspaceOpenRequest {
        root: path_string(root.path()),
        workspace: path_string(workspace.path()),
        settings: settings(),
    })
    .await
    .unwrap();
    let entry_file = entry_file_path(workspace.path());
    write_text(
        &entry_file,
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
        ),
    );

    let entries = workspace_inspect_entries(WorkspaceInspectEntriesRequest {
        workspace: path_string(workspace.path()),
        file: None,
        status: InspectEntryStatusInput::Memory,
        limit: 10,
        offset: 0,
    })
    .unwrap();
    assert_eq!(entries.total, 1);
    assert_eq!(entries.entries[0].source, "Steel Sword");
    assert_eq!(entries.entries[0].translation.as_deref(), Some("钢剑"));
    assert_eq!(
        entries.entries[0].translation_meta.as_ref().unwrap()["origin"],
        "memory"
    );

    let diagnostics = workspace_inspect_diagnostics(WorkspaceInspectDiagnosticsRequest {
        workspace: path_string(workspace.path()),
        file: None,
        severity: InspectDiagnosticSeverityInput::Warning,
        limit: 10,
        offset: 0,
    })
    .unwrap();
    assert_eq!(diagnostics.total, 1);
    assert_eq!(
        diagnostics.diagnostics[0].entry_id,
        "scaleform:Interface/Translations/MyMod_English.txt:$Desc"
    );
    assert_eq!(
        diagnostics.diagnostics[0].diagnostic["code"],
        "memory.conflict"
    );
}

fn settings() -> SettingsInput {
    SettingsInput {
        game_release: Some("SkyrimSe".to_string()),
        asset_language: Some("English".to_string()),
        source_locale: Some("en".to_string()),
        target_locale: Some("zh-Hans".to_string()),
    }
}

fn write_text(path: &std::path::Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn path_string(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn entry_file_path(workspace: &std::path::Path) -> std::path::PathBuf {
    let workspace_json: Value =
        serde_json::from_str(&fs::read_to_string(workspace.join("workspace.json")).unwrap())
            .unwrap();
    workspace.join(workspace_json["files"][0]["path"].as_str().unwrap())
}

struct TempRoot {
    path: std::path::PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "stringer_app_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn eet_v1_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"EET_");
    push_i32(&mut bytes, 1);
    push_u32(&mut bytes, 1);
    bytes.extend_from_slice(b"LINE");
    push_u32(&mut bytes, 1);
    push_u32_string(&mut bytes, "WEAP");
    push_u32_string(&mut bytes, "00001234");
    push_u32_string(&mut bytes, "IronSword");
    push_u32_string(&mut bytes, "FULL");
    push_u32_string(&mut bytes, "Iron Sword");
    push_u32_string(&mut bytes, "铁剑");
    push_u32_string(&mut bytes, "");
    push_i32(&mut bytes, 1);
    bytes.extend_from_slice(&99i16.to_le_bytes());
    push_i32(&mut bytes, 42);
    push_u32_string(&mut bytes, "");
    bytes
}

fn push_u32_string(bytes: &mut Vec<u8>, value: &str) {
    let data = value.as_bytes();
    push_u32(bytes, data.len() as u32);
    bytes.extend_from_slice(data);
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
