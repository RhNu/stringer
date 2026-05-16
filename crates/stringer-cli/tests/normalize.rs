use std::fs;
use std::process::Command as ProcessCommand;

use serde_json::Value;

#[test]
fn workspace_normalize_json_dry_run_and_human_apply() {
    let root = TempRoot::new("normalize-root");
    let workspace = TempRoot::new("normalize-workspace");
    let asset = root
        .path()
        .join("Data/Interface/Translations/MyMod_English.txt");
    write_text(&asset, "$Title\tSteel Sword\n");

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            &path_string(root.path()),
            "--workspace",
            &path_string(workspace.path()),
            "--force",
            "--game-release",
            "SkyrimSe",
            "--asset-language",
            "English",
            "--source-locale",
            "en",
            "--target-locale",
            "zh-Hans",
        ])
        .output()
        .unwrap();
    assert_success(&open);

    let entry_path = entry_file_path(workspace.path());
    write_text(
        &entry_path,
        "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
    );
    let rules = workspace.path().join("rules.txt");
    write_text(&rules, "StartRule\nSearch=钢剑\nReplace=熟铁剑\nEndRule\n");

    let dry_run = stringer_command()
        .args([
            "workspace",
            "normalize",
            "--workspace",
            &path_string(workspace.path()),
            "--rules",
            &path_string(&rules),
            "--encoding",
            "utf-8",
            "--json",
        ])
        .output()
        .unwrap();
    assert_success(&dry_run);
    let summary: Value = serde_json::from_slice(&dry_run.stdout).unwrap();
    assert_eq!(summary["changed_entries"], 1);
    assert_eq!(summary["total_replacements"], 1);
    assert_eq!(summary["changes"][0]["before"], "钢剑");
    assert_eq!(summary["changes"][0]["after"], "熟铁剑");
    assert_eq!(read_entry(&entry_path)["translation"], "钢剑");

    let applied = stringer_command()
        .args([
            "workspace",
            "normalize",
            "--workspace",
            &path_string(workspace.path()),
            "--rules",
            &path_string(&rules),
            "--encoding",
            "utf-8",
            "--apply",
        ])
        .output()
        .unwrap();
    assert_success(&applied);
    let stdout = String::from_utf8(applied.stdout).unwrap();
    assert!(stdout.contains("applied normalization"));
    assert!(stdout.contains("stringer knowledge validate"));
    let row = read_entry(&entry_path);
    assert_eq!(row["translation"], "熟铁剑");
    assert_eq!(row["translation_meta"]["origin"], "agent");
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stringer_command() -> ProcessCommand {
    let mut command = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"));
    command.env("STRINGER_CONFIG", isolated_config_path());
    command
}

fn isolated_config_path() -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!(
            "stringer_cli_normalize_isolated_config_{}",
            std::process::id()
        ))
        .join("config.toml")
}

fn write_text(path: &std::path::Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read_entry(path: &std::path::Path) -> Value {
    serde_json::from_str(fs::read_to_string(path).unwrap().trim()).unwrap()
}

fn entry_file_path(workspace: &std::path::Path) -> std::path::PathBuf {
    let workspace_json: Value =
        serde_json::from_str(&fs::read_to_string(workspace.join("workspace.json")).unwrap())
            .unwrap();
    workspace.join(workspace_json["files"][0]["path"].as_str().unwrap())
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
            "stringer_cli_{label}_{}_{}",
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
