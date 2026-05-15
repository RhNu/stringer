use std::fs;
use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};

use clap::{CommandFactory, Parser};
use serde_json::Value;
use stringer_cli::{
    AdaptCommand, AdaptFormatArg, Cli, Command, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeLookupFieldArg, KnowledgeLookupSourceArg, WorkspaceBatchCommand, WorkspaceCommand,
};

#[test]
fn workspace_open_command_uses_root_workspace_and_settings() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "open",
        "--root",
        "input",
        "--workspace",
        "translations",
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
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.workspace.as_str(), "translations");
    assert_eq!(command.game_release.as_deref(), Some("SkyrimSe"));
    assert_eq!(command.asset_language.as_deref(), Some("English"));
    assert_eq!(command.source_locale.as_deref(), Some("en"));
    assert_eq!(command.target_locale.as_deref(), Some("zh-Hans"));
}

#[test]
fn workspace_finalize_command_uses_root_workspace_and_override_root_paths() {
    let cli = Cli::parse_from([
        "stringer",
        "workspace",
        "finalize",
        "--root",
        "input",
        "--workspace",
        "translations",
        "--override-root",
        "override",
    ]);

    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Finalize(command) = command else {
        panic!("expected workspace finalize command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.workspace.as_str(), "translations");
    assert_eq!(command.override_root.as_str(), "override");
}

#[test]
fn workspace_batch_commands_parse_workspace_file_limit_input_and_batch_id() {
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
        "apply",
        "--workspace",
        "translations",
        "--input",
        "-",
    ]);
    let Command::Workspace { command } = cli.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Batch { command } = command else {
        panic!("expected workspace batch command");
    };
    let WorkspaceBatchCommand::Apply(command) = command else {
        panic!("expected workspace batch apply command");
    };
    assert_eq!(command.input.as_str(), "-");

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

#[test]
fn adapt_import_command_rejects_global_knowledge_root_override() {
    let result = Cli::try_parse_from([
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
        "--global-knowledge-root",
        "global-knowledge",
    ]);

    assert!(result.is_err());
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
fn workspace_batch_claim_emits_json_and_apply_reads_stdin() {
    let root = test_path("cli-batch-root");
    let translations = test_path("cli-batch-translations");
    let asset = root.join("Data/Interface/Translations/MyMod_English.txt");
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    fs::write(&asset, "$Title\tIron Sword\n").unwrap();

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

    let claim = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
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
    let entry_id = claim_json["entries"][0]["id"].as_str().unwrap();

    let patch = serde_json::json!({
        "batch_id": batch_id,
        "entries": [
            { "id": entry_id, "translation": "熟铁剑" }
        ]
    })
    .to_string();
    let mut apply = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "workspace",
            "batch",
            "apply",
            "--workspace",
            translations.as_str(),
            "--input",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    apply
        .stdin
        .as_mut()
        .unwrap()
        .write_all(patch.as_bytes())
        .unwrap();
    let apply = apply.wait_with_output().unwrap();
    assert!(apply.status.success());
    let summary: Value = serde_json::from_slice(&apply.stdout).unwrap();
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

#[test]
fn knowledge_annotate_command_uses_project_root_workspace_and_skip_fill_flag() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--project-root",
        "input",
        "--workspace",
        "translations",
        "--skip-fill-memory",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Annotate(command) = command else {
        panic!("expected knowledge annotate command");
    };
    assert_eq!(command.project_root.as_deref(), Some("input".into()));
    assert_eq!(command.workspace.as_str(), "translations");
    assert!(command.skip_fill_memory);
}

#[test]
fn knowledge_annotate_accepts_project_root_and_workspace() {
    let result = Cli::try_parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--project-root",
        "input",
        "--workspace",
        "translations",
        "--skip-fill-memory",
    ]);

    assert!(result.is_ok());
}

#[test]
fn knowledge_annotate_rejects_removed_auto_fill_memory_flag() {
    let result = Cli::try_parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--workspace",
        "translations",
        "--auto-fill-memory",
    ]);

    assert!(result.is_err());
}

#[test]
fn knowledge_annotate_accepts_current_directory_project_root_default() {
    let result = Cli::try_parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--workspace",
        "translations",
    ]);

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
fn knowledge_commands_reject_cli_knowledge_roots() {
    for flag in ["--global-knowledge-root", "--override-knowledge-root"] {
        let result = Cli::try_parse_from([
            "stringer",
            "knowledge",
            "lookup",
            "--project-root",
            "input",
            "--text",
            "Iron Sword",
            flag,
            "knowledge",
        ]);

        assert!(result.is_err());
    }
}

#[test]
fn knowledge_validate_command_uses_project_root_and_workspace() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "validate",
        "--project-root",
        "input",
        "--workspace",
        "translations",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Validate(command) = command else {
        panic!("expected knowledge validate command");
    };
    assert_eq!(command.project_root.as_deref(), Some("input".into()));
    assert_eq!(command.workspace.as_str(), "translations");
}

#[test]
fn knowledge_lookup_command_uses_text_context_settings_and_json_flag() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "lookup",
        "--project-root",
        "input",
        "--text",
        "Iron Sword",
        "--kind",
        "plugin",
        "--record-type",
        "WEAP",
        "--subrecord",
        "FULL",
        "--game-release",
        "SkyrimSe",
        "--asset-language",
        "English",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
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
    assert_eq!(command.project_root.as_deref(), Some("input".into()));
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
fn knowledge_lookup_uses_current_directory_as_default_project_root() {
    let project = test_path("cli-lookup-current-project");
    let terms = project.join("knowledge/terms/base.toml");
    fs::create_dir_all(terms.parent().unwrap()).unwrap();
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

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "knowledge",
            "lookup",
            "--text",
            "Stringer Current Root",
            "--game-release",
            "SkyrimSe",
            "--asset-language",
            "English",
            "--source-locale",
            "en",
            "--target-locale",
            "zh-Hans",
            "--json",
        ])
        .current_dir(project)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"total_matches\": 1"));
    assert!(stdout.contains("当前目录"));
}

#[test]
fn knowledge_index_rebuild_command_uses_project_root_and_settings() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "index",
        "rebuild",
        "--project-root",
        "input",
        "--game-release",
        "SkyrimSe",
        "--asset-language",
        "English",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Index { command } = command else {
        panic!("expected knowledge index command");
    };
    let KnowledgeIndexCommand::Rebuild(command) = command;
    assert_eq!(command.project_root.as_deref(), Some("input".into()));
}

#[test]
fn root_help_explains_agent_workflow() {
    let help = Cli::command().render_long_help().to_string();

    assert!(help.contains("Typical workflow"));
    assert!(help.contains("workspace open"));
    assert!(help.contains("workspace finalize"));
    assert!(help.contains("workspace batch"));
    assert!(help.contains("adapt import"));
    assert!(help.contains("knowledge annotate"));
    assert!(help.contains("entries/**/*.jsonl"));
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
    assert!(help.contains("--override-root"));
    assert!(help.contains("override directory"));
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
