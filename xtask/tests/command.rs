use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn help_lists_clap_subcommands() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("line-budget"));
    assert!(stdout.contains("release"));
}

#[test]
fn line_budget_command_uses_the_default_limit() {
    let workspace = TestWorkspace::new("line_budget_command");

    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("line-budget")
        .current_dir(workspace.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("at or below 850 lines"));
}

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("stringer_xtask_{name}_{unique}"));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
