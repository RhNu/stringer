use std::fs;
use std::process::Command as ProcessCommand;

use clap::Parser;
use stringer_cli::{Cli, ProgressModeArg};

#[test]
fn global_feedback_flags_parse_before_subcommand() {
    let cli = Cli::parse_from([
        "stringer",
        "-vv",
        "--progress",
        "never",
        "knowledge",
        "annotate",
    ]);

    assert_eq!(cli.verbose, 2);
    assert!(!cli.quiet);
    assert_eq!(cli.progress, ProgressModeArg::Never);
}

#[tokio::test]
async fn quiet_rejects_forced_progress() {
    let cli = Cli::parse_from([
        "stringer",
        "--quiet",
        "--progress",
        "always",
        "workspace",
        "batch",
        "count",
    ]);

    let error = stringer_cli::run(cli).await.unwrap_err();

    assert!(
        error
            .to_string()
            .contains("--quiet cannot be used with --progress always")
    );
}

#[test]
fn progress_always_writes_status_to_stderr_without_changing_stdout() {
    let workspace = empty_workspace("cli-progress-workspace");

    let output = stringer_command()
        .args([
            "--progress",
            "always",
            "workspace",
            "batch",
            "count",
            "--workspace",
            workspace.as_str(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "counted 0 entries: 0 claimable, 0 empty, 0 memory-prefilled, 0 translated, 0 skipped, 0 claimed, 0 with diagnostics\n"
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("done: workspace batch count")
    );
}

#[test]
fn progress_always_writes_annotate_progress_to_captured_stderr() {
    let (source_root, workspace) = translation_workspace("cli-progress-annotate");

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            source_root.as_str(),
            "--workspace",
            workspace.as_str(),
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

    let output = stringer_command()
        .args([
            "--progress",
            "always",
            "knowledge",
            "annotate",
            "--workspace",
            workspace.as_str(),
        ])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("done: knowledge annotate")
    );
}

#[test]
fn progress_auto_does_not_write_status_to_captured_stderr() {
    let workspace = empty_workspace("cli-progress-auto-workspace");

    let output = stringer_command()
        .args([
            "workspace",
            "batch",
            "count",
            "--workspace",
            workspace.as_str(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn verbose_emits_tracing_to_stderr_without_progress() {
    let (source_root, workspace) = translation_workspace("cli-verbose");

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            source_root.as_str(),
            "--workspace",
            workspace.as_str(),
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

    let output = stringer_command()
        .args([
            "-v",
            "--progress",
            "never",
            "knowledge",
            "annotate",
            "--workspace",
            workspace.as_str(),
        ])
        .env_remove("RUST_LOG")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("starting knowledge annotation"));
    assert!(stderr.contains("finished knowledge annotation"));
}

#[test]
fn rust_log_overrides_verbose_default_tracing() {
    let (source_root, workspace) = translation_workspace("cli-verbose-rust-log");

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            source_root.as_str(),
            "--workspace",
            workspace.as_str(),
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

    let output = stringer_command()
        .args([
            "-v",
            "--progress",
            "never",
            "knowledge",
            "annotate",
            "--workspace",
            workspace.as_str(),
        ])
        .env("RUST_LOG", "error")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn quiet_suppresses_rust_log_tracing() {
    let (source_root, workspace) = translation_workspace("cli-quiet-rust-log");

    let open = stringer_command()
        .args([
            "workspace",
            "open",
            "--source-root",
            source_root.as_str(),
            "--workspace",
            workspace.as_str(),
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

    let output = stringer_command()
        .args([
            "--quiet",
            "--progress",
            "never",
            "knowledge",
            "annotate",
            "--workspace",
            workspace.as_str(),
        ])
        .env("RUST_LOG", "debug")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

fn empty_workspace(name: &str) -> camino::Utf8PathBuf {
    let workspace = test_path(name);
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("workspace.json"),
        r#"{"schema_version":4,"kind":"stringer.workspace","source_root":"C:/source","game_release":"SkyrimSe","asset_language":"English","source_locale":"en","target_locale":"zh-Hans","files":[]}"#,
    )
    .unwrap();
    workspace
}

fn translation_workspace(name: &str) -> (camino::Utf8PathBuf, camino::Utf8PathBuf) {
    let source_root = test_path(&format!("{name}-source"));
    let workspace = test_path(&format!("{name}-workspace"));
    fs::create_dir_all(source_root.join("Data/Interface/Translations")).unwrap();
    fs::write(
        source_root.join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    )
    .unwrap();
    (source_root, workspace)
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
