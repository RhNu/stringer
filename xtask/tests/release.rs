use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::{
    PathAppendCopyOutcome, copy_release_binary_to_path_append_out_path, release_binary_path,
};

#[test]
fn release_binary_path_points_to_the_stringer_release_executable() {
    let workspace = TestWorkspace::new("release_binary_path");

    let path = release_binary_path(workspace.path());

    assert_eq!(
        path,
        workspace
            .path()
            .join("target")
            .join("release")
            .join(format!("stringer{}", std::env::consts::EXE_SUFFIX))
    );
}

#[test]
fn copies_release_binary_when_path_append_out_path_is_an_existing_directory() {
    let workspace = TestWorkspace::new("release_copy");
    let out = TestWorkspace::new("release_out");
    let binary = workspace.write_file(
        &format!("target/release/stringer{}", std::env::consts::EXE_SUFFIX),
        "compiled binary",
    );
    let expected_destination = out
        .path()
        .join(format!("stringer{}", std::env::consts::EXE_SUFFIX));

    let outcome =
        copy_release_binary_to_path_append_out_path(&binary, Some(out.path().to_path_buf()))
            .unwrap();

    assert_eq!(
        outcome,
        PathAppendCopyOutcome::Copied(expected_destination.clone())
    );
    assert_eq!(
        fs::read_to_string(expected_destination).unwrap(),
        "compiled binary"
    );
}

#[test]
fn skips_release_binary_copy_when_path_append_out_path_is_not_defined() {
    let workspace = TestWorkspace::new("release_skip_missing_env");
    let binary = workspace.write_file(
        &format!("target/release/stringer{}", std::env::consts::EXE_SUFFIX),
        "compiled binary",
    );

    let outcome = copy_release_binary_to_path_append_out_path(&binary, None).unwrap();

    assert_eq!(outcome, PathAppendCopyOutcome::SkippedMissingEnv);
}

#[test]
fn skips_release_binary_copy_when_path_append_out_path_does_not_exist() {
    let workspace = TestWorkspace::new("release_skip_missing_dir");
    let binary = workspace.write_file(
        &format!("target/release/stringer{}", std::env::consts::EXE_SUFFIX),
        "compiled binary",
    );
    let missing_out = workspace.path().join("missing");

    let outcome =
        copy_release_binary_to_path_append_out_path(&binary, Some(missing_out.clone())).unwrap();

    assert_eq!(
        outcome,
        PathAppendCopyOutcome::SkippedMissingDirectory(missing_out)
    );
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

    fn write_file(&self, relative_path: &str, contents: &str) -> PathBuf {
        let path = self.path.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
