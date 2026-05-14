use clap::Parser;
use stringer_cli::{Cli, Command, KnowledgeCommand};

#[test]
fn export_command_uses_root_and_out_paths() {
    let cli = Cli::parse_from([
        "stringer",
        "export",
        "--root",
        "input",
        "--out",
        "translations",
    ]);

    let Command::Export(command) = cli.command else {
        panic!("expected export command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.out.as_str(), "translations");
}

#[test]
fn import_command_uses_root_translations_and_override_root_paths() {
    let cli = Cli::parse_from([
        "stringer",
        "import",
        "--root",
        "input",
        "--translations",
        "translations",
        "--override-root",
        "override",
    ]);

    let Command::Import(command) = cli.command else {
        panic!("expected import command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.translations.as_str(), "translations");
    assert_eq!(command.override_root.as_str(), "override");
}

#[test]
fn export_command_does_not_define_config_override_flag() {
    let error = Cli::try_parse_from([
        "stringer",
        "export",
        "--root",
        "input",
        "--out",
        "translations",
        "--config",
        "config.toml",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("unexpected argument '--config'"));
}

#[test]
fn import_command_does_not_define_settings_flags() {
    let error = Cli::try_parse_from([
        "stringer",
        "import",
        "--root",
        "input",
        "--translations",
        "translations",
        "--override-root",
        "override",
        "--game-release",
        "SkyrimSe",
    ])
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unexpected argument '--game-release'")
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
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Validate(command) = command else {
        panic!("expected knowledge validate command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.translations.as_str(), "translations");
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
    assert!(command.json);
}
