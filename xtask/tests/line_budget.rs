use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::find_line_budget_violations;

#[test]
fn reports_rs_files_that_exceed_the_line_budget_under_crates() {
    let workspace = TestWorkspace::new("line_budget_exceeded");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\nthree\n");
    workspace.write_file("crates/demo/README.md", "one\ntwo\nthree\nfour\n");
    workspace.write_file("outside.rs", "one\ntwo\nthree\nfour\n");

    let violations = find_line_budget_violations(workspace.path(), 2).unwrap();

    assert_eq!(violations.len(), 1);
    assert_eq!(
        violations[0].path,
        workspace.path().join("crates/demo/src/lib.rs")
    );
    assert_eq!(violations[0].line_count, 3);
}

#[test]
fn accepts_rs_files_at_or_below_the_line_budget_under_crates() {
    let workspace = TestWorkspace::new("line_budget_within_limit");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\n");
    workspace.write_file("crates/demo/tests/integration.rs", "one\n");

    let violations = find_line_budget_violations(workspace.path(), 2).unwrap();

    assert!(violations.is_empty());
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

    fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
