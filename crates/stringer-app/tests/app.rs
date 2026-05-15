use std::fs;

use serde_json::Value;
use stringer_app::{
    AdaptFormatInput, AdaptImportRequest, InspectDiagnosticSeverityInput, InspectEntryStatusInput,
    KnowledgeTermInput, KnowledgeTermStatusInput, KnowledgeTermUpsertRequest, SettingsInput,
    WorkspaceBatchApplyEntry, WorkspaceBatchApplyRequest, WorkspaceBatchClaimRequest,
    WorkspaceBatchCountRequest, WorkspaceFinalizeRequest, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectEntriesRequest, WorkspaceOpenRequest, adapt_import, knowledge_term_upsert,
    workspace_batch_apply, workspace_batch_claim, workspace_batch_count, workspace_finalize,
    workspace_inspect_diagnostics, workspace_inspect_entries, workspace_open,
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
        source_root: path_string(root.path()),
        workspace: Some(path_string(workspace.path())),
        force: false,
        settings: settings(),
    })
    .await
    .unwrap();
    assert_eq!(opened.entries, 1);

    let count = workspace_batch_count(WorkspaceBatchCountRequest {
        workspace: Some(path_string(workspace.path())),
        file: None,
    })
    .unwrap();
    assert_eq!(count.total, 1);
    assert_eq!(count.empty, 1);

    let claim = workspace_batch_claim(WorkspaceBatchClaimRequest {
        workspace: Some(path_string(workspace.path())),
        file: None,
        limit: 1,
    })
    .unwrap();
    let batch_id = claim.batch_id.expect("batch id");
    let entry_id = claim.entries[0].id.clone();
    assert_eq!(claim.entries[0].source, "Iron Sword");

    let summary = workspace_batch_apply(WorkspaceBatchApplyRequest {
        workspace: Some(path_string(workspace.path())),
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
async fn app_workspace_open_reads_workspace_config_not_source_config() {
    let root = TempRoot::new("workspace-settings-root");
    let workspace = TempRoot::new("workspace-settings-output");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    write_text(
        &root.path().join("stringer.toml"),
        r#"
game_release = "SkyrimLe"
asset_language = "Chinese"
source_locale = "fr"
target_locale = "de"
"#,
    );
    write_text(
        &workspace.path().join("stringer.toml"),
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );

    workspace_open(WorkspaceOpenRequest {
        source_root: path_string(root.path()),
        workspace: Some(path_string(workspace.path())),
        force: false,
        settings: SettingsInput::default(),
    })
    .await
    .unwrap();

    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(workspace.path().join("workspace.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["game_release"], "SkyrimSe");
    assert_eq!(manifest["asset_language"], "English");
    assert_eq!(manifest["source_locale"], "en");
    assert_eq!(manifest["target_locale"], "zh-Hans");
}

#[tokio::test]
async fn app_workspace_finalize_defaults_output_under_workspace() {
    let root = TempRoot::new("workspace-finalize-root");
    let workspace = TempRoot::new("workspace-finalize-output");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );

    workspace_open(WorkspaceOpenRequest {
        source_root: path_string(root.path()),
        workspace: Some(path_string(workspace.path())),
        force: false,
        settings: settings(),
    })
    .await
    .unwrap();
    write_text(
        &entry_file_path(workspace.path()),
        r#"{"id":"scaleform:Interface/Translations/MyMod_English.txt:$Title","translation":"熟铁剑"}"#,
    );

    let finalized = workspace_finalize(WorkspaceFinalizeRequest {
        workspace: Some(path_string(workspace.path())),
        source_root: None,
        output: None,
    })
    .await
    .unwrap();

    assert_eq!(finalized.applied_entries, 1);
    assert!(
        workspace
            .path()
            .join("output/Data/Interface/Translations/MyMod_English.txt")
            .exists()
    );
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
async fn app_knowledge_term_upsert_can_prepare_workspace_knowledge_before_open() {
    let workspace = TempRoot::new("term-before-open");

    let summary = knowledge_term_upsert(KnowledgeTermUpsertRequest {
        workspace: Some(path_string(workspace.path())),
        file: None,
        terms: vec![KnowledgeTermInput {
            id: "term:iron_sword".to_string(),
            source: "Iron Sword".to_string(),
            target: "熟铁剑".to_string(),
            aliases: Vec::new(),
            case_sensitive: false,
            status: KnowledgeTermStatusInput::Preferred,
            scope: Default::default(),
            tags: Vec::new(),
            note: None,
        }],
        rebuild_index: false,
    })
    .unwrap();

    assert_eq!(summary.action, "upserted");
    assert_eq!(
        summary.path,
        path_string(&workspace.path().join("knowledge/terms/workspace.toml"))
    );
    assert!(!workspace.path().join("workspace.json").exists());
    assert!(
        workspace
            .path()
            .join("knowledge/terms/workspace.toml")
            .exists()
    );
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
        source_root: path_string(root.path()),
        workspace: Some(path_string(workspace.path())),
        force: false,
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
        workspace: Some(path_string(workspace.path())),
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
        workspace: Some(path_string(workspace.path())),
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
