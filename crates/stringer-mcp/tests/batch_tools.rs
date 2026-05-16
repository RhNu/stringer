use std::fs;
use std::sync::Once;

use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientInfo},
};
use serde_json::{Value, json};
use stringer_mcp::StringerMcp;

#[tokio::test]
async fn mcp_workspace_batch_claim_read_detail_and_submit_use_compact_packets() {
    let root = TempRoot::new("mcp-batch-root");
    let workspace = TempRoot::new("mcp-batch-workspace");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n$Desc\tSteel Sword\n");

    let (client, server_handle) = connect().await;
    client
        .call_tool(
            CallToolRequestParams::new("workspace_open").with_arguments(args(json!({
                "source_root": path_string(root.path()),
                "workspace": path_string(workspace.path()),
                "settings": {
                    "game_release": "SkyrimSe",
                    "asset_language": "English",
                    "source_locale": "en",
                    "target_locale": "zh-Hans"
                }
            }))),
        )
        .await
        .unwrap();

    let claim = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_claim").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "limit": 2
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(claim["claimed_entries"], 2);
    assert!(claim.get("entries").is_none());
    let batch_id = claim["batch_id"].as_str().unwrap();
    let revision = claim["revision"].as_u64().unwrap();

    let page = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_read").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "offset": 0,
                "limit": 1
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(page["total_entries"], 2);
    assert_eq!(page["revision"], revision);
    assert_eq!(page["next_offset"], 1);
    assert!(matches!(
        page["entries"][0]["source"].as_str(),
        Some("Iron Sword" | "Steel Sword")
    ));
    assert!(page["entries"][0].get("id").is_none());
    let key = page["entries"][0]["key"].as_str().unwrap();

    let detail = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_detail").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "keys": [key]
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(detail["entries"][0]["key"], key);
    assert!(
        detail["entries"][0]["id"]
            .as_str()
            .unwrap()
            .contains("scaleform:")
    );

    let applied = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": key, "action": "translate", "translation": "熟铁剑" }]
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(applied["applied_entries"], 1);
    assert_eq!(applied["remaining_entries"], 1);
    let revision = applied["revision"].as_u64().unwrap();

    let remaining = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_read").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let skipped_key = remaining["entries"][0]["key"].as_str().unwrap();
    let skipped = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": skipped_key, "action": "skip", "skip_reason": "not_translatable" }]
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(skipped["applied_entries"], 1);
    assert_eq!(skipped["remaining_entries"], 0);

    let count = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_count").with_arguments(args(json!({
                "workspace": path_string(workspace.path())
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(count["skipped"], 1);

    let empty_claim = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_claim").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "limit": 1
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(empty_claim["claimed_entries"], 0);

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn mcp_workspace_batch_submit_errors_explain_stale_revisions() {
    let root = TempRoot::new("mcp-batch-stale-id-root");
    let workspace = TempRoot::new("mcp-batch-stale-id-workspace");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n$Desc\tSteel Sword\n");

    let (client, server_handle) = connect().await;
    client
        .call_tool(
            CallToolRequestParams::new("workspace_open").with_arguments(args(json!({
                "source_root": path_string(root.path()),
                "workspace": path_string(workspace.path()),
                "settings": {
                    "game_release": "SkyrimSe",
                    "asset_language": "English",
                    "source_locale": "en",
                    "target_locale": "zh-Hans"
                }
            }))),
        )
        .await
        .unwrap();

    let claim = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_claim").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "limit": 2
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let batch_id = claim["batch_id"].as_str().unwrap();
    let revision = claim["revision"].as_u64().unwrap();
    let page = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_read").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "offset": 0,
                "limit": 1
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let key = page["entries"][0]["key"].as_str().unwrap();

    client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": key, "action": "translate", "translation": "熟铁剑" }]
            }))),
        )
        .await
        .unwrap();
    let error = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": key, "action": "translate", "translation": "熟铁剑" }]
            }))),
        )
        .await
        .unwrap_err();
    let rmcp::ServiceError::McpError(error) = error else {
        panic!("expected MCP error, got {error:?}");
    };
    let data: Value = error.data.unwrap();

    assert_eq!(data["code"], "workspace.batch_revision_conflict");
    assert!(
        data["message"]
            .as_str()
            .unwrap()
            .contains("revision conflict")
    );
    assert_eq!(data["details"]["batch_id"], batch_id);
    assert_eq!(data["details"]["current_revision"], 2);
    assert_eq!(data["details"]["recovery"], "read_batch_before_retrying");

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn mcp_workspace_batch_submit_errors_explain_completed_batch_ids() {
    let root = TempRoot::new("mcp-batch-completed-root");
    let workspace = TempRoot::new("mcp-batch-completed-workspace");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n");

    let (client, server_handle) = connect().await;
    client
        .call_tool(
            CallToolRequestParams::new("workspace_open").with_arguments(args(json!({
                "source_root": path_string(root.path()),
                "workspace": path_string(workspace.path()),
                "settings": {
                    "game_release": "SkyrimSe",
                    "asset_language": "English",
                    "source_locale": "en",
                    "target_locale": "zh-Hans"
                }
            }))),
        )
        .await
        .unwrap();

    let claim = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_claim").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "limit": 1
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let batch_id = claim["batch_id"].as_str().unwrap();
    let revision = claim["revision"].as_u64().unwrap();
    let page = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_read").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id
            }))),
        )
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let key = page["entries"][0]["key"].as_str().unwrap();

    client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": key, "action": "translate", "translation": "熟铁剑" }]
            }))),
        )
        .await
        .unwrap();
    let error = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_submit").with_arguments(args(json!({
                "workspace": path_string(workspace.path()),
                "batch_id": batch_id,
                "revision": revision,
                "entries": [{ "key": key, "action": "translate", "translation": "熟铁剑" }]
            }))),
        )
        .await
        .unwrap_err();
    let rmcp::ServiceError::McpError(error) = error else {
        panic!("expected MCP error, got {error:?}");
    };
    let data: Value = error.data.unwrap();

    assert_eq!(data["code"], "workspace.batch_not_found");
    assert!(
        data["message"]
            .as_str()
            .unwrap()
            .contains("claim a fresh batch")
    );
    assert_eq!(data["details"]["batch_id"], batch_id);
    assert_eq!(data["details"]["recovery"], "claim_fresh_batch");

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

fn args(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().unwrap().clone()
}

async fn connect() -> (
    rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    tokio::task::JoinHandle<()>,
) {
    isolate_global_knowledge();
    let (server_transport, client_transport) = tokio::io::duplex(8192);
    let server_handle = tokio::spawn(async move {
        StringerMcp
            .serve(server_transport)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = TestClient.serve(client_transport).await.unwrap();
    (client, server_handle)
}

fn isolate_global_knowledge() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let config = std::env::temp_dir()
            .join(format!(
                "stringer_mcp_isolated_config_{}",
                std::process::id()
            ))
            .join("config.toml");
        // SAFETY: MCP tests set the same process-wide config path once before
        // starting any in-process MCP service that reads Stringer settings.
        unsafe {
            std::env::set_var("STRINGER_CONFIG", config);
        }
    });
}

#[derive(Debug, Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

fn write_text(path: &std::path::Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn path_string(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

struct TempRoot {
    path: std::path::PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "stringer_mcp_{label}_{}_{}",
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
