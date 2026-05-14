use clap::Parser;
use stringer_cli::{Cli, Command};

#[test]
fn export_command_uses_root_and_out_paths() {
    let cli = Cli::parse_from([
        "stringer",
        "export",
        "--root",
        "input",
        "--out",
        "out.jsonl",
    ]);

    let Command::Export(command) = cli.command else {
        panic!("expected export command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.out.as_str(), "out.jsonl");
}

#[test]
fn import_command_uses_root_translations_and_override_root_paths() {
    let cli = Cli::parse_from([
        "stringer",
        "import",
        "--root",
        "input",
        "--translations",
        "translations.jsonl",
        "--override-root",
        "override",
    ]);

    let Command::Import(command) = cli.command else {
        panic!("expected import command");
    };
    assert_eq!(command.root.as_str(), "input");
    assert_eq!(command.translations.as_str(), "translations.jsonl");
    assert_eq!(command.override_root.as_str(), "override");
}

#[test]
fn cli_does_not_define_config_override_flag() {
    let error = Cli::try_parse_from([
        "stringer",
        "export",
        "--root",
        "input",
        "--out",
        "out.jsonl",
        "--config",
        "config.toml",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("unexpected argument '--config'"));
}
