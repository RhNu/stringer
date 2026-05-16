use std::fs;
use std::process::Command as ProcessCommand;

use clap::{CommandFactory, Parser};
use serde_json::Value;
use stringer_cli::{
    AdaptCommand, AdaptFormatArg, Cli, Command, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeLookupFieldArg, KnowledgeLookupSourceArg, WorkspaceCommand,
    WorkspaceNormalizeEncodingArg,
};

#[test]
fn workspace_open_command_uses_source_root_default_workspace_force_and_settings() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "open",
        "--source-root",
        "input",
        "--force",
        "--game-release",
        "SkyrimSe",
        "--asset-language",
        "English",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Open(command) = command else {
        panic!("expected workspace open command");
    };
    assert_eq!(command.source_root.as_str(), "input");
    assert_eq!(command.workspace.as_str(), ".");
    assert!(command.force);
    assert_eq!(command.game_release.as_deref(), Some("SkyrimSe"));
    assert_eq!(command.asset_language.as_deref(), Some("English"));
    assert_eq!(command.source_locale.as_deref(), Some("en"));
    assert_eq!(command.target_locale.as_deref(), Some("zh-Hans"));
}

#[test]
fn workspace_finalize_command_defaults_workspace_source_override_and_output() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "finalize",
        "--source-root",
        "input",
        "--output",
        "override",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Finalize(command) = command else {
        panic!("expected workspace finalize command");
    };
    assert_eq!(command.workspace.as_str(), ".");
    assert_eq!(command.source_root.as_deref(), Some("input".into()));
    assert_eq!(command.output.as_deref(), Some("override".into()));
}

#[test]
fn workspace_normalize_command_parses_rules_apply_encoding_limit_and_json() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "normalize",
        "--workspace",
        "translations",
        "--rules",
        "rules.txt",
        "--file",
        "entries/scaleform/MyMod.jsonl",
        "--apply",
        "--encoding",
        "cp936",
        "--limit",
        "5",
        "--json",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Normalize(command) = command else {
        panic!("expected workspace normalize command");
    };
    assert_eq!(command.workspace.as_str(), "translations");
    assert_eq!(command.rules.as_str(), "rules.txt");
    assert_eq!(
        command.file.as_deref(),
        Some("entries/scaleform/MyMod.jsonl")
    );
    assert!(command.apply);
    assert_eq!(command.encoding, WorkspaceNormalizeEncodingArg::Cp936);
    assert_eq!(command.limit, 5);
    assert!(command.json);
}

#[test]
fn workspace_commands_reject_removed_path_flags() {
    assert!(Cli::try_parse_from(["stringer", "workspace", "open", "--root", "input"]).is_err());
    assert!(
        Cli::try_parse_from([
            "stringer",
            "workspace",
            "finalize",
            "--override-root",
            "out"
        ])
        .is_err()
    );
}

#[test]
fn workspace_upgrade_command_is_placeholder() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "upgrade",
        "--workspace",
        "translations",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Upgrade(command) = command else {
        panic!("expected workspace upgrade command");
    };
    assert_eq!(command.workspace.as_str(), "translations");
}

#[test]
fn top_level_export_command_is_removed() {
    let mut command = Cli::command();
    assert!(command.find_subcommand_mut("export").is_none());

    let result = Cli::try_parse_from([
        "stringer",
        "export",
        "--root",
        "input",
        "--workspace",
        "translations",
    ]);

    assert!(result.is_err());
}

#[test]
fn top_level_import_command_is_removed() {
    let mut command = Cli::command();
    assert!(command.find_subcommand_mut("import").is_none());

    let result = Cli::try_parse_from([
        "stringer",
        "import",
        "--root",
        "input",
        "--workspace",
        "translations",
        "--override-root",
        "override",
    ]);

    assert!(result.is_err());
}

#[test]
fn adapt_import_command_uses_format_input_output_and_locales() {
    let cli = Cli::parse_from([
        "stringer",
        "adapt",
        "import",
        "--format",
        "xt-sst",
        "--input",
        "source.sst",
        "--out",
        "knowledge/memory/source.jsonl",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
        "--game",
        "SkyrimSe",
    ]);

    let Command::Adapt { command } = cli.command else {
        panic!("expected adapt command");
    };
    let AdaptCommand::Import(command) = command;
    assert_eq!(command.format, AdaptFormatArg::XtSst);
    assert_eq!(command.input.as_str(), "source.sst");
    assert_eq!(
        command.out.as_deref(),
        Some("knowledge/memory/source.jsonl".into())
    );
    assert_eq!(command.source_locale, "en");
    assert_eq!(command.target_locale, "zh-Hans");
    assert_eq!(command.game.as_deref(), Some("SkyrimSe"));
}

#[test]
fn adapt_import_command_can_default_to_global_memory() {
    let cli = Cli::parse_from([
        "stringer",
        "adapt",
        "import",
        "--format",
        "xt-sst",
        "--input",
        "source.sst",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
    ]);

    let Command::Adapt { command } = cli.command else {
        panic!("expected adapt command");
    };
    let AdaptCommand::Import(command) = command;
    assert_eq!(command.format, AdaptFormatArg::XtSst);
    assert_eq!(command.input.as_str(), "source.sst");
    assert_eq!(command.out, None);
}

#[tokio::test]
async fn adapt_import_command_writes_memory_jsonl() {
    let input = test_path("cli-adapt.eet");
    let output = test_path("cli-adapt-memory.jsonl");
    fs::write(&input, eet_v1_fixture()).unwrap();

    let cli = Cli::parse_from([
        "stringer",
        "adapt",
        "import",
        "--format",
        "eet",
        "--input",
        input.as_str(),
        "--out",
        output.as_str(),
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
        "--game",
        "skyrim-se",
    ]);

    stringer_cli::run(cli).await.unwrap();

    let line = fs::read_to_string(output).unwrap();
    let row: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(row["source"], "Iron Sword");
    assert_eq!(row["target"], "铁剑");
    assert_eq!(row["source_locale"], "en");
    assert_eq!(row["target_locale"], "zh-Hans");
    assert_eq!(row["context"]["record_type"], "WEAP");
    assert_eq!(row["context"]["game"], "SkyrimSe");
}

#[test]
fn knowledge_annotate_command_defaults_workspace_and_uses_skip_fill_flag() {
    let cli = Cli::parse_from(["stringer", "knowledge", "annotate", "--skip-fill-memory"]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Annotate(command) = command else {
        panic!("expected knowledge annotate command");
    };
    assert_eq!(command.workspace.as_str(), ".");
    assert!(command.skip_fill_memory);
}

#[test]
fn knowledge_commands_reject_project_root() {
    let result = Cli::try_parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--project-root",
        "input",
    ]);

    assert!(result.is_err());
}

#[test]
fn knowledge_annotate_rejects_removed_auto_fill_memory_flag() {
    let result = Cli::try_parse_from(["stringer", "knowledge", "annotate", "--auto-fill-memory"]);

    assert!(result.is_err());
}

#[test]
fn knowledge_annotate_accepts_current_directory_workspace_default() {
    let result = Cli::try_parse_from(["stringer", "knowledge", "annotate"]);

    assert!(result.is_ok());
}

#[test]
fn knowledge_annotate_rejects_old_root_and_translations_flags() {
    for flag in ["--root", "--translations"] {
        let args = if flag == "--root" {
            [
                "stringer",
                "knowledge",
                "annotate",
                "--root",
                "input",
                "--workspace",
                "translations",
            ]
        } else {
            [
                "stringer",
                "knowledge",
                "annotate",
                "--project-root",
                "input",
                "--translations",
                "translations",
            ]
        };

        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn knowledge_validate_command_defaults_workspace() {
    let cli = Cli::parse_from(["stringer", "knowledge", "validate"]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Validate(command) = command else {
        panic!("expected knowledge validate command");
    };
    assert_eq!(command.workspace.as_str(), ".");
}

#[test]
fn knowledge_lookup_command_uses_text_context_settings_and_json_flag() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "lookup",
        "--text",
        "Iron Sword",
        "--kind",
        "plugin",
        "--record-type",
        "WEAP",
        "--subrecord",
        "FULL",
        "--regex",
        "--limit",
        "5",
        "--case-sensitive",
        "--source",
        "memory",
        "--field",
        "target",
        "--json",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Lookup(command) = command else {
        panic!("expected knowledge lookup command");
    };
    assert_eq!(command.workspace.as_str(), ".");
    assert_eq!(command.text, "Iron Sword");
    assert_eq!(command.kind, "plugin");
    assert_eq!(command.record_type.as_deref(), Some("WEAP"));
    assert_eq!(command.subrecord.as_deref(), Some("FULL"));
    assert!(command.regex);
    assert_eq!(command.limit, 5);
    assert!(command.case_sensitive);
    assert_eq!(command.source, KnowledgeLookupSourceArg::Memory);
    assert_eq!(command.field, KnowledgeLookupFieldArg::Target);
    assert!(command.json);
}

#[test]
fn knowledge_lookup_command_defaults_to_agent_search_options() {
    let cli = Cli::parse_from(["stringer", "knowledge", "lookup", "--text", "altmer"]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Lookup(command) = command else {
        panic!("expected knowledge lookup command");
    };
    assert_eq!(command.limit, 20);
    assert_eq!(command.source, KnowledgeLookupSourceArg::All);
    assert_eq!(command.field, KnowledgeLookupFieldArg::Both);
    assert!(!command.regex);
    assert!(!command.case_sensitive);
}

#[test]
fn knowledge_lookup_uses_current_directory_as_default_workspace() {
    let workspace = test_path("cli-lookup-current-workspace");
    let terms = workspace.join("knowledge/terms/workspace.toml");
    fs::create_dir_all(terms.parent().unwrap()).unwrap();
    fs::write(
        workspace.join("workspace.json"),
        r#"{"schema_version":4,"kind":"stringer.workspace","source_root":"C:/source","game_release":"SkyrimSe","asset_language":"English","source_locale":"en","target_locale":"zh-Hans","files":[]}"#,
    )
    .unwrap();
    fs::write(
        &terms,
        r#"
[[terms]]
id = "term:stringer-current"
source = "Stringer Current Root"
target = "当前目录"
status = "preferred"
"#,
    )
    .unwrap();

    let output = stringer_command()
        .args([
            "knowledge",
            "lookup",
            "--text",
            "Stringer Current Root",
            "--json",
        ])
        .current_dir(workspace)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"total_matches\": 1"));
    assert!(stdout.contains("当前目录"));
}

#[test]
fn knowledge_index_rebuild_command_defaults_workspace() {
    let cli = Cli::parse_from(["stringer", "knowledge", "index", "rebuild"]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Index { command } = command else {
        panic!("expected knowledge index command");
    };
    let KnowledgeIndexCommand::Rebuild(command) = command;
    assert_eq!(command.workspace.as_str(), ".");
}

#[test]
fn root_help_points_to_compact_agent_workflow() {
    let help = Cli::command().render_long_help().to_string();

    assert!(help.contains("Typical workflow"));
    assert!(help.contains("workspace open"));
    assert!(help.contains("--source-root"));
    assert!(help.contains("workspace finalize"));
    assert!(help.contains("workspace batch"));
    assert!(help.contains("adapt import"));
    assert!(help.contains("knowledge annotate"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--quiet"));
    assert!(help.contains("--verbose"));
    assert!(help.contains("entries/**/*.jsonl"));
    assert!(help.contains("skills/stringer-workflows"));
    assert!(!help.contains("Recommended agent workflow"));
    assert!(!help.contains("Default knowledge locations"));
    assert!(!help.contains("stringer export --root"));
    assert!(!help.contains("stringer import --root"));
}

#[test]
fn workspace_open_help_explains_workspace_output() {
    let mut command = Cli::command();
    let help = command
        .find_subcommand_mut("workspace")
        .expect("workspace subcommand exists")
        .find_subcommand_mut("open")
        .expect("workspace open subcommand exists")
        .render_long_help()
        .to_string();

    assert!(help.contains("workspace.json"));
    assert!(help.contains("batches"));
    assert!(help.contains("--workspace"));
    assert!(help.contains("--source-root"));
    assert!(help.contains("--game-release"));
    assert!(help.contains("source-locale"));
}

#[test]
fn workspace_finalize_help_explains_override_output() {
    let mut command = Cli::command();
    let help = command
        .find_subcommand_mut("workspace")
        .expect("workspace subcommand exists")
        .find_subcommand_mut("finalize")
        .expect("workspace finalize subcommand exists")
        .render_long_help()
        .to_string();

    assert!(help.contains("--workspace"));
    assert!(help.contains("--output"));
    assert!(help.contains("output directory"));
    assert!(!help.contains("--override-root"));
}

#[test]
fn adapt_import_help_explains_memory_conversion() {
    let mut command = Cli::command();
    let adapt = command
        .find_subcommand_mut("adapt")
        .expect("adapt subcommand exists");
    let help = adapt
        .find_subcommand_mut("import")
        .expect("adapt import subcommand exists")
        .render_long_help()
        .to_string();

    assert!(help.contains("translation memory JSONL"));
    assert!(help.contains("xt-sst"));
    assert!(help.contains("--source-locale"));
}

#[test]
fn lookup_help_explains_machine_readable_usage() {
    let mut command = Cli::command();
    let knowledge = command
        .find_subcommand_mut("knowledge")
        .expect("knowledge subcommand exists");
    let help = knowledge
        .find_subcommand_mut("lookup")
        .expect("lookup subcommand exists")
        .render_long_help()
        .to_string();

    assert!(help.contains("--json"));
    assert!(help.contains("--regex"));
    assert!(help.contains("--limit"));
    assert!(help.contains("--source"));
    assert!(help.contains("--field"));
    assert!(help.contains("plugin"));
    assert!(help.contains("Altmer"));
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
