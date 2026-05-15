use std::fs;

use clap::{CommandFactory, Parser};
use serde_json::Value;
use stringer_cli::{
    AdaptCommand, AdaptFormatArg, Cli, Command, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeLookupFieldArg, KnowledgeLookupSourceArg, WorkspaceCommand,
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
        "--global-knowledge-root",
        "global-knowledge",
    ]);

    let Command::Adapt { command } = cli.command else {
        panic!("expected adapt command");
    };
    let AdaptCommand::Import(command) = command;
    assert_eq!(command.format, AdaptFormatArg::XtSst);
    assert_eq!(command.input.as_str(), "source.sst");
    assert_eq!(command.out, None);
    assert_eq!(
        command.global_knowledge_root.as_deref(),
        Some("global-knowledge".into())
    );
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

#[tokio::test]
async fn adapt_import_command_merges_into_source_named_global_memory_by_default() {
    let input = test_path("cli-adapt-source.eet");
    let global = test_path("cli-adapt-global");
    fs::write(&input, eet_v1_fixture()).unwrap();

    let args = [
        "stringer",
        "adapt",
        "import",
        "--format",
        "eet",
        "--input",
        input.as_str(),
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
        "--game",
        "skyrim-se",
        "--global-knowledge-root",
        global.as_str(),
    ];

    stringer_cli::run(Cli::parse_from(args)).await.unwrap();
    stringer_cli::run(Cli::parse_from(args)).await.unwrap();

    let output = global
        .join("memory")
        .join("adapt")
        .join(format!("{}.jsonl", input.file_name().unwrap()));
    let text = fs::read_to_string(output).unwrap();
    let rows = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["source"], "Iron Sword");
    assert_eq!(rows[0]["target"], "铁剑");
}

#[tokio::test]
async fn adapt_import_command_keeps_different_sources_in_separate_global_memory_files() {
    let first = test_path("first-source.eet");
    let second = test_path("second-source.eet");
    let global = test_path("separate-global");
    fs::write(&first, eet_v1_fixture()).unwrap();
    fs::write(&second, eet_v1_fixture()).unwrap();

    for input in [&first, &second] {
        let cli = Cli::parse_from([
            "stringer",
            "adapt",
            "import",
            "--format",
            "eet",
            "--input",
            input.as_str(),
            "--source-locale",
            "en",
            "--target-locale",
            "zh-Hans",
            "--global-knowledge-root",
            global.as_str(),
        ]);
        stringer_cli::run(cli).await.unwrap();
    }

    assert!(
        global
            .join("memory")
            .join("adapt")
            .join(format!("{}.jsonl", first.file_name().unwrap()))
            .exists()
    );
    assert!(
        global
            .join("memory")
            .join("adapt")
            .join(format!("{}.jsonl", second.file_name().unwrap()))
            .exists()
    );
}

#[test]
fn knowledge_annotate_command_uses_root_translations_and_auto_fill_flag() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "annotate",
        "--root",
        "input",
        "--translations",
        "translations",
        "--auto-fill-memory",
        "--global-knowledge-root",
        "global",
        "--override-knowledge-root",
        "override",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Annotate(command) = command else {
        panic!("expected knowledge annotate command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.translations.as_str(), "translations");
    assert!(command.auto_fill_memory);
    assert_eq!(
        command.global_knowledge_root.as_deref(),
        Some("global".into())
    );
    assert_eq!(
        command.override_knowledge_root.as_deref(),
        Some("override".into())
    );
}

#[test]
fn knowledge_validate_command_uses_root_and_translations() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "validate",
        "--root",
        "input",
        "--translations",
        "translations",
        "--global-knowledge-root",
        "global",
        "--override-knowledge-root",
        "override",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Validate(command) = command else {
        panic!("expected knowledge validate command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.translations.as_str(), "translations");
    assert_eq!(
        command.global_knowledge_root.as_deref(),
        Some("global".into())
    );
    assert_eq!(
        command.override_knowledge_root.as_deref(),
        Some("override".into())
    );
}

#[test]
fn knowledge_lookup_command_uses_text_context_settings_and_json_flag() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "lookup",
        "--root",
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
        "--global-knowledge-root",
        "global",
        "--override-knowledge-root",
        "override",
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
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.text, "Iron Sword");
    assert_eq!(command.kind, "plugin");
    assert_eq!(command.record_type.as_deref(), Some("WEAP"));
    assert_eq!(command.subrecord.as_deref(), Some("FULL"));
    assert_eq!(
        command.global_knowledge_root.as_deref(),
        Some("global".into())
    );
    assert_eq!(
        command.override_knowledge_root.as_deref(),
        Some("override".into())
    );
    assert!(command.regex);
    assert_eq!(command.limit, 5);
    assert!(command.case_sensitive);
    assert_eq!(command.source, KnowledgeLookupSourceArg::Memory);
    assert_eq!(command.field, KnowledgeLookupFieldArg::Target);
    assert!(command.json);
}

#[test]
fn knowledge_lookup_command_defaults_to_agent_search_options() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "lookup",
        "--root",
        "input",
        "--text",
        "altmer",
    ]);

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
fn knowledge_index_rebuild_command_uses_root_settings_and_knowledge_roots() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "index",
        "rebuild",
        "--root",
        "input",
        "--game-release",
        "SkyrimSe",
        "--asset-language",
        "English",
        "--source-locale",
        "en",
        "--target-locale",
        "zh-Hans",
        "--global-knowledge-root",
        "global",
        "--override-knowledge-root",
        "override",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Index { command } = command else {
        panic!("expected knowledge index command");
    };
    let KnowledgeIndexCommand::Rebuild(command) = command;
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(
        command.global_knowledge_root.as_deref(),
        Some("global".into())
    );
    assert_eq!(
        command.override_knowledge_root.as_deref(),
        Some("override".into())
    );
}

#[test]
fn root_help_explains_agent_workflow() {
    let help = Cli::command().render_long_help().to_string();

    assert!(help.contains("Typical workflow"));
    assert!(help.contains("workspace open"));
    assert!(help.contains("workspace finalize"));
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

    assert!(help.contains("manifest.json"));
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
