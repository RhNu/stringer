use std::fs;
use std::process::Command as ProcessCommand;

use clap::Parser;
use serde_json::Value;
use stringer_cli::{Cli, Command, WorkspaceCommand, WorkspaceInspectCommand};

#[test]
fn workspace_inspect_commands_parse_agent_read_flags() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "inspect",
        "entries",
        "--workspace",
        "translations",
        "--file",
        "entries/scaleform/asset.jsonl",
        "--status",
        "diagnostic",
        "--limit",
        "25",
        "--offset",
        "5",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Inspect { command } = command else {
        panic!("expected workspace inspect command");
    };
    let WorkspaceInspectCommand::Entries(command) = command else {
        panic!("expected inspect entries command");
    };
    assert_eq!(command.workspace.as_str(), "translations");
    assert_eq!(
        command.file.as_deref(),
        Some("entries/scaleform/asset.jsonl")
    );
    assert_eq!(command.status.to_string(), "diagnostic");
    assert_eq!(command.limit, 25);
    assert_eq!(command.offset, 5);

    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "inspect",
        "diagnostics",
        "--workspace",
        "translations",
        "--severity",
        "warning",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Inspect { command } = command else {
        panic!("expected workspace inspect command");
    };
    let WorkspaceInspectCommand::Diagnostics(command) = command else {
        panic!("expected inspect diagnostics command");
    };
    assert_eq!(command.severity.to_string(), "warning");
    assert_eq!(command.limit, 50);
    assert_eq!(command.offset, 0);
}

#[test]
fn workspace_inspect_entries_and_diagnostics_emit_json() {
    let root = test_path("inspect-root");
    let translations = test_path("inspect-translations");
    let asset = root.join("Data/Interface/Translations/MyMod_English.txt");
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    fs::write(&asset, "$Title\tIron Sword\n$Desc\tSteel Sword\n").unwrap();

    let open = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "workspace",
            "open",
            "--root",
            root.as_str(),
            "--workspace",
            translations.as_str(),
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
    assert!(open.status.success());

    let workspace: Value =
        serde_json::from_str(&fs::read_to_string(translations.join("workspace.json")).unwrap())
            .unwrap();
    let entry_path = translations.join(workspace["files"][0]["path"].as_str().unwrap());
    fs::write(
        &entry_path,
        concat!(
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Title\",\"source\":\"Iron Sword\"}\n",
            "{\"id\":\"scaleform:Interface/Translations/MyMod_English.txt:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"},\"diagnostics\":[{\"severity\":\"warning\",\"code\":\"memory.conflict\",\"message\":\"check\"}]}\n",
        ),
    )
    .unwrap();

    let entries = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "workspace",
            "inspect",
            "entries",
            "--workspace",
            translations.as_str(),
            "--status",
            "memory",
        ])
        .output()
        .unwrap();
    assert!(entries.status.success());
    let entries_json: Value = serde_json::from_slice(&entries.stdout).unwrap();
    assert_eq!(entries_json["total"], 1);
    assert_eq!(entries_json["entries"][0]["source"], "Steel Sword");

    let diagnostics = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "workspace",
            "inspect",
            "diagnostics",
            "--workspace",
            translations.as_str(),
            "--severity",
            "warning",
        ])
        .output()
        .unwrap();
    assert!(diagnostics.status.success());
    let diagnostics_json: Value = serde_json::from_slice(&diagnostics.stdout).unwrap();
    assert_eq!(diagnostics_json["total"], 1);
    assert_eq!(
        diagnostics_json["diagnostics"][0]["diagnostic"]["code"],
        "memory.conflict"
    );

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(translations);
}

fn test_path(name: &str) -> camino::Utf8PathBuf {
    camino::Utf8PathBuf::from_path_buf(
        std::env::temp_dir().join(format!("stringer-cli-{}-{name}", std::process::id())),
    )
    .unwrap()
}
