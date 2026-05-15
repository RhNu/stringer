use std::fs;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use serde_json::Value;
use stringer_cli::{Cli, Command, KnowledgeCommand, KnowledgeTermCommand, KnowledgeTermStatusArg};

#[test]
fn knowledge_term_upsert_command_parses_core_flags() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "term",
        "upsert",
        "--project-root",
        "project",
        "--file",
        "knowledge/terms/weapons.toml",
        "--id",
        "term:iron_sword",
        "--source",
        "Iron Sword",
        "--target",
        "熟铁剑",
        "--status",
        "forbidden",
        "--alias",
        "Iron Blade",
        "--case-sensitive",
        "--scope-json",
        r#"{"game":["SkyrimSe"],"kind":["plugin"]}"#,
        "--tag",
        "weapon",
        "--note",
        "Project wording",
        "--rebuild-index",
        "--json",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Term { command } = command else {
        panic!("expected knowledge term command");
    };
    let KnowledgeTermCommand::Upsert(command) = command else {
        panic!("expected knowledge term upsert command");
    };

    assert_eq!(command.project_root.as_deref(), Some("project".into()));
    assert_eq!(
        command.file.as_deref(),
        Some("knowledge/terms/weapons.toml".into())
    );
    assert_eq!(command.id, "term:iron_sword");
    assert_eq!(command.source, "Iron Sword");
    assert_eq!(command.target, "熟铁剑");
    assert_eq!(command.status, KnowledgeTermStatusArg::Forbidden);
    assert_eq!(command.aliases, ["Iron Blade"]);
    assert!(command.case_sensitive);
    assert_eq!(
        command.scope_json.as_deref(),
        Some(r#"{"game":["SkyrimSe"],"kind":["plugin"]}"#)
    );
    assert_eq!(command.tags, ["weapon"]);
    assert_eq!(command.note.as_deref(), Some("Project wording"));
    assert!(command.rebuild_index);
    assert!(command.json);
}

#[test]
fn knowledge_term_delete_command_parses_core_flags() {
    let cli = Cli::parse_from([
        "stringer",
        "knowledge",
        "term",
        "delete",
        "--project-root",
        "project",
        "--id",
        "term:iron_sword",
        "--rebuild-index",
        "--json",
    ]);

    let Command::Knowledge { command } = cli.command else {
        panic!("expected knowledge command");
    };
    let KnowledgeCommand::Term { command } = command else {
        panic!("expected knowledge term command");
    };
    let KnowledgeTermCommand::Delete(command) = command else {
        panic!("expected knowledge term delete command");
    };

    assert_eq!(command.project_root.as_deref(), Some("project".into()));
    assert_eq!(command.file, None);
    assert_eq!(command.id, "term:iron_sword");
    assert!(command.rebuild_index);
    assert!(command.json);
}

#[test]
fn knowledge_term_upsert_and_delete_integrate_with_lookup() {
    let project = TempRoot::new("cli-knowledge-term");

    let upsert = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "knowledge",
            "term",
            "upsert",
            "--project-root",
            project.path.to_str().unwrap(),
            "--id",
            "term:iron_sword",
            "--source",
            "Iron Sword",
            "--target",
            "熟铁剑",
            "--scope-json",
            r#"{"game":["SkyrimSe"],"kind":["plugin"]}"#,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(upsert.status.success(), "{}", stderr(&upsert));
    let upsert_json: Value = serde_json::from_slice(&upsert.stdout).unwrap();
    assert_eq!(upsert_json["action"], "upserted");
    assert_eq!(upsert_json["id"], "term:iron_sword");

    let lookup = lookup_iron_sword(&project);
    assert!(lookup.status.success(), "{}", stderr(&lookup));
    let lookup_json: Value = serde_json::from_slice(&lookup.stdout).unwrap();
    assert_eq!(lookup_json["total_matches"], 1);
    assert_eq!(lookup_json["results"][0]["target"], "熟铁剑");

    let delete = ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "knowledge",
            "term",
            "delete",
            "--project-root",
            project.path.to_str().unwrap(),
            "--id",
            "term:iron_sword",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(delete.status.success(), "{}", stderr(&delete));
    let delete_json: Value = serde_json::from_slice(&delete.stdout).unwrap();
    assert_eq!(delete_json["action"], "deleted");

    let lookup = lookup_iron_sword(&project);
    assert!(lookup.status.success(), "{}", stderr(&lookup));
    let lookup_json: Value = serde_json::from_slice(&lookup.stdout).unwrap();
    assert_eq!(lookup_json["total_matches"], 0);
}

fn lookup_iron_sword(project: &TempRoot) -> std::process::Output {
    ProcessCommand::new(env!("CARGO_BIN_EXE_stringer"))
        .args([
            "knowledge",
            "lookup",
            "--project-root",
            project.path.to_str().unwrap(),
            "--text",
            "Iron Sword",
            "--source",
            "terms",
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
        .output()
        .unwrap()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

struct TempRoot {
    path: std::path::PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "stringer_cli_{label}_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
