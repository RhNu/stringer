use std::fs;
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use clap::Parser;
use serde_json::Value;
use stringer_cli::{Cli, Command, WorkspaceBatchCommand, WorkspaceCommand};

#[test]
fn workspace_batch_commands_parse_packet_workflow_flags() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "batch",
        "claim",
        "--workspace",
        "translations",
        "--file",
        "entries/plugin/MyMod.esp/WEAP.jsonl",
        "--limit",
        "25",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Claim(command) = command else {
        panic!("expected workspace batch claim command");
    };
    assert_eq!(command.workspace.as_str(), "translations");
    assert_eq!(
        command.file.as_deref(),
        Some("entries/plugin/MyMod.esp/WEAP.jsonl")
    );
    assert_eq!(command.limit, 25);

    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "batch",
        "read",
        "--batch-id",
        "b123-4",
        "--limit",
        "10",
        "--offset",
        "20",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Read(command) = command else {
        panic!("expected workspace batch read command");
    };
    assert_eq!(command.batch_id, "b123-4");
    assert_eq!(command.limit, 10);
    assert_eq!(command.offset, 20);

    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "batch",
        "detail",
        "--batch-id",
        "b123-4",
        "--key",
        "e001",
        "--key",
        "e002",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Detail(command) = command else {
        panic!("expected workspace batch detail command");
    };
    assert_eq!(command.keys, vec!["e001", "e002"]);

    let cli = Cli::parse_from(["stringer", "workspace", "batch", "submit", "--input", "-"]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Submit(command) = command else {
        panic!("expected workspace batch submit command");
    };
    assert_eq!(command.workspace.as_str(), ".");
    assert_eq!(command.input.as_str(), "-");

    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "batch",
        "export",
        "--batch-id",
        "b123-4",
        "--out",
        "work",
        "--format",
        "csv",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Export(command) = command else {
        panic!("expected workspace batch export command");
    };
    assert_eq!(command.batch_id, "b123-4");
    assert_eq!(command.out.unwrap().as_str(), "work");
    assert_eq!(command.format.to_string(), "csv");

    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "batch",
        "release",
        "--workspace",
        "translations",
        "--batch-id",
        "b123-4",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Release(command) = command else {
        panic!("expected workspace batch release command");
    };
    assert_eq!(command.batch_id, "b123-4");

    assert!(
        Cli::try_parse_from(["stringer", "workspace", "batch", "apply", "--input", "-"]).is_err()
    );
}

#[test]
fn workspace_batch_claim_emits_json_and_submit_reads_stdin() {
    let root = test_path("cli-batch-root");
    let translations = test_path("cli-batch-translations");
    let asset = root.join("Data/Interface/Translations/MyMod_English.txt");
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    fs::write(&asset, "$Title\tIron Sword\n").unwrap();

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            root.as_str(),
            "--workspace",
            translations.as_str(),
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
    assert!(open.status.success());

    let claim = stringer_command()
        .args([
            "workspace",
            "batch",
            "claim",
            "--workspace",
            translations.as_str(),
            "--limit",
            "1",
        ])
        .output()
        .unwrap();
    assert!(claim.status.success());
    let claim_json: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let batch_id = claim_json["batch_id"].as_str().unwrap();
    assert_eq!(claim_json["claimed_entries"], 1);
    assert!(claim_json.get("entries").is_none());

    let read = stringer_command()
        .args([
            "workspace",
            "batch",
            "read",
            "--workspace",
            translations.as_str(),
            "--batch-id",
            batch_id,
            "--limit",
            "1",
        ])
        .output()
        .unwrap();
    assert!(read.status.success());
    let read_json: Value = serde_json::from_slice(&read.stdout).unwrap();
    assert_eq!(read_json["total_entries"], 1);
    let key = read_json["entries"][0]["key"].as_str().unwrap();
    let revision = read_json["revision"].as_u64().unwrap();

    let patch = serde_json::json!({
        "batch_id": batch_id,
        "revision": revision,
        "entries": [
            { "key": key, "action": "translate", "translation": "熟铁剑" }
        ]
    })
    .to_string();
    let mut submit = stringer_command()
        .args([
            "workspace",
            "batch",
            "submit",
            "--workspace",
            translations.as_str(),
            "--input",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    submit
        .stdin
        .as_mut()
        .unwrap()
        .write_all(patch.as_bytes())
        .unwrap();
    let submit = submit.wait_with_output().unwrap();
    assert!(submit.status.success());
    let summary: Value = serde_json::from_slice(&submit.stdout).unwrap();
    assert_eq!(summary["applied_entries"], 1);

    let workspace: Value =
        serde_json::from_str(&fs::read_to_string(translations.join("workspace.json")).unwrap())
            .unwrap();
    let entry_path = translations.join(workspace["files"][0]["path"].as_str().unwrap());
    let row: Value = serde_json::from_str(fs::read_to_string(entry_path).unwrap().trim()).unwrap();
    assert_eq!(row["translation"], "熟铁剑");
    assert_eq!(row["translation_meta"]["origin"], "agent");

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(translations);
}

fn test_path(name: &str) -> camino::Utf8PathBuf {
    camino::Utf8PathBuf::from_path_buf(
        std::env::temp_dir().join(format!("stringer-cli-{}-{name}", std::process::id())),
    )
    .unwrap()
}

fn stringer_command() -> ProcessCommand {
    let mut command = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"));
    command.env("STRINGER_CONFIG", isolated_config_path());
    command
}

fn isolated_config_path() -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!(
            "stringer_cli_isolated_config_{}",
            std::process::id()
        ))
        .join("config.toml")
}
