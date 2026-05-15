use std::fs;

use rmcp::{
    ClientHandler, ServiceExt,
    model::{CallToolRequestParams, ClientInfo},
};
use serde_json::{Value, json};
use stringer_mcp::StringerMcp;

#[tokio::test]
async fn mcp_lists_cli_equivalent_tools_with_object_output_schemas() {
    let (client, server_handle) = connect().await;

    let tools = client.peer().list_tools(None).await.unwrap();
    let mut names: Vec<&str> = tools.tools.iter().map(|tool| tool.name.as_ref()).collect();
    names.sort_unstable();

    assert_eq!(
        names,
        [
            "adapt_import",
            "knowledge_annotate",
            "knowledge_index_rebuild",
            "knowledge_lookup",
            "knowledge_validate",
            "workspace_batch_apply",
            "workspace_batch_claim",
            "workspace_batch_count",
            "workspace_batch_release",
            "workspace_finalize",
            "workspace_open",
        ]
    );
    assert!(!names.contains(&"run_command"));
    for tool in &tools.tools {
        assert_schema_has_no_uint_format(&tool.input_schema);
        let output_schema = tool.output_schema.as_ref().expect("output schema");
        assert_schema_has_no_uint_format(output_schema);
        assert_eq!(
            output_schema.get("type").and_then(Value::as_str),
            Some("object")
        );
    }

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn mcp_workspace_open_returns_structured_content_through_tool_call() {
    let root = TempRoot::new("mcp-root");
    let workspace = TempRoot::new("mcp-workspace");
    let asset = root
        .path()
        .join("Data")
        .join("Interface")
        .join("Translations")
        .join("MyMod_English.txt");
    write_text(&asset, "$Title\tIron Sword\n");

    let (client, server_handle) = connect().await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("workspace_open").with_arguments(args(json!({
                "root": path_string(root.path()),
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

    assert_eq!(result.structured_content.unwrap()["entries"], 1);

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

#[tokio::test]
async fn mcp_errors_include_app_code_message_and_details() {
    let (client, server_handle) = connect().await;
    let error = client
        .call_tool(
            CallToolRequestParams::new("workspace_batch_count").with_arguments(args(json!({
                "workspace": "missing-workspace"
            }))),
        )
        .await
        .unwrap_err();
    let rmcp::ServiceError::McpError(error) = error else {
        panic!("expected MCP error, got {error:?}");
    };
    let data: Value = error.data.unwrap();

    assert_eq!(data["code"], "workspace.read_file");
    assert!(data["message"].as_str().unwrap().contains("failed to read"));
    assert_eq!(data["details"]["path"], "missing-workspace/workspace.json");

    client.cancel().await.unwrap();
    server_handle.await.unwrap();
}

fn args(value: Value) -> serde_json::Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn assert_schema_has_no_uint_format(schema: &serde_json::Map<String, Value>) {
    assert_value_has_no_uint_format(&Value::Object(schema.clone()));
}

fn assert_value_has_no_uint_format(value: &Value) {
    match value {
        Value::Object(object) => {
            if let Some(format) = object.get("format").and_then(Value::as_str) {
                assert!(
                    !format.starts_with("uint"),
                    "schema contains unsigned integer format `{format}`"
                );
            }
            for value in object.values() {
                assert_value_has_no_uint_format(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                assert_value_has_no_uint_format(value);
            }
        }
        _ => {}
    }
}

async fn connect() -> (
    rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    tokio::task::JoinHandle<()>,
) {
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
