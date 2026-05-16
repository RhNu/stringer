use std::fs;
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use serde_json::Value;

#[test]
fn workspace_batch_submit_can_skip_entries_without_writing_translation() {
    let root = test_path("cli-batch-skip-root");
    let translations = test_path("cli-batch-skip-translations");
    let asset = root.join("Data/Interface/Translations/MyMod_English.txt");
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    fs::write(&asset, "$Title\tIron Sword\n").unwrap();

    assert_success(
        stringer_command()
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
            .unwrap(),
    );

    let claim = assert_success(
        stringer_command()
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
            .unwrap(),
    );
    let claim_json: Value = serde_json::from_slice(&claim.stdout).unwrap();
    let batch_id = claim_json["batch_id"].as_str().unwrap();

    let read = assert_success(
        stringer_command()
            .args([
                "workspace",
                "batch",
                "read",
                "--workspace",
                translations.as_str(),
                "--batch-id",
                batch_id,
            ])
            .output()
            .unwrap(),
    );
    let read_json: Value = serde_json::from_slice(&read.stdout).unwrap();
    let key = read_json["entries"][0]["key"].as_str().unwrap();
    let revision = read_json["revision"].as_u64().unwrap();

    let patch = serde_json::json!({
        "batch_id": batch_id,
        "revision": revision,
        "entries": [{ "key": key, "action": "skip", "skip_reason": "not_translatable" }]
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
    let submit = assert_success(submit.wait_with_output().unwrap());
    let summary: Value = serde_json::from_slice(&submit.stdout).unwrap();
    assert_eq!(summary["applied_entries"], 1);
    assert_eq!(summary["remaining_entries"], 0);

    let workspace: Value =
        serde_json::from_str(&fs::read_to_string(translations.join("workspace.json")).unwrap())
            .unwrap();
    let entry_path = translations.join(workspace["files"][0]["path"].as_str().unwrap());
    let row: Value = serde_json::from_str(fs::read_to_string(entry_path).unwrap().trim()).unwrap();
    assert!(row.get("translation").is_none());
    assert_eq!(row["translation_meta"]["origin"], "skipped");
    assert_eq!(row["translation_meta"]["skip_reason"], "not_translatable");

    let count = assert_success(
        stringer_command()
            .args([
                "workspace",
                "batch",
                "count",
                "--workspace",
                translations.as_str(),
                "--json",
            ])
            .output()
            .unwrap(),
    );
    let count_json: Value = serde_json::from_slice(&count.stdout).unwrap();
    assert_eq!(count_json["empty"], 0);
    assert_eq!(count_json["skipped"], 1);

    let reclaim = assert_success(
        stringer_command()
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
            .unwrap(),
    );
    let reclaim_json: Value = serde_json::from_slice(&reclaim.stdout).unwrap();
    assert_eq!(reclaim_json["claimed_entries"], 0);
    assert!(reclaim_json["batch_id"].is_null());

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(translations);
}

fn assert_success(output: std::process::Output) -> std::process::Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
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
