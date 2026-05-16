use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use camino::Utf8PathBuf;
use serde_json::Value;

pub const ENTRY_FILE: &str = "entries/scaleform/MyMod.jsonl";

pub struct WorkspaceFixture {
    _root: TempRoot,
    workspace: PathBuf,
}

impl WorkspaceFixture {
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
}

pub fn workspace_with_rows(label: &str, rows: &str) -> WorkspaceFixture {
    let root = TempRoot::new(label);
    let workspace = root.path().join("workspace");
    write_text(&workspace.join(ENTRY_FILE), rows);
    write_text(
        &workspace.join("workspace.json"),
        r#"{
  "schema_version": 4,
  "kind": "stringer.workspace",
  "source_root": "C:/Source/MyMod",
  "game_release": "SkyrimSe",
  "asset_language": "English",
  "source_locale": "en",
  "target_locale": "zh-Hans",
  "files": [
    {
      "path": "entries/scaleform/MyMod.jsonl",
      "kind": "scaleform",
      "asset_path": "Interface/Translations/MyMod_English.txt"
    }
  ]
}
"#,
    );
    fs::create_dir_all(workspace.join("batches")).unwrap();
    WorkspaceFixture {
        _root: root,
        workspace,
    }
}

pub fn write_batch(workspace: &Path, batch_id: &str, entry_ids: &[&str]) {
    let ids = entry_ids
        .iter()
        .map(|id| format!(r#""{id}""#))
        .collect::<Vec<_>>()
        .join(",");
    write_text(
        &workspace.join("batches").join(format!("{batch_id}.json")),
        &format!(
            r#"{{
  "schema_version": 4,
  "batch_id": "{batch_id}",
  "created_at_unix_ms": 1,
  "scope": {{"file": "entries/scaleform/MyMod.jsonl"}},
  "entry_ids": [{ids}]
}}
"#
        ),
    );
}

pub fn rows() -> &'static str {
    concat!(
        "{\"id\":\"scaleform:MyMod:$Title\",\"source\":\"Iron Sword\"}\n",
        "{\"id\":\"scaleform:MyMod:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
        "{\"id\":\"scaleform:MyMod:$Done\",\"source\":\"Done\",\"translation\":\"完成\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
        "{\"id\":\"scaleform:MyMod:$Warn\",\"source\":\"Needs Review\",\"diagnostics\":[{\"severity\":\"info\",\"code\":\"review.note\",\"message\":\"inspect\"}]}\n",
    )
}

#[allow(dead_code)]
pub fn jsonl_rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub fn utf8(path: &Path) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(path.to_path_buf()).unwrap()
}

fn write_text(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    pub fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "stringer_workspace_ops_{label}_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
