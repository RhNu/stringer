use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Eq, PartialEq)]
pub enum PathAppendCopyOutcome {
    Copied(PathBuf),
    SkippedMissingEnv,
    SkippedMissingDirectory(PathBuf),
}

pub fn release_build_args() -> [&'static str; 6] {
    [
        "build",
        "-p",
        "stringer-cli",
        "-p",
        "stringer-mcp",
        "--release",
    ]
}

pub fn release_binary_path(workspace_root: impl AsRef<Path>) -> PathBuf {
    release_executable_path(workspace_root, "stringer")
}

pub fn release_binary_paths(workspace_root: impl AsRef<Path>) -> [PathBuf; 2] {
    let workspace_root = workspace_root.as_ref();
    [
        release_executable_path(workspace_root, "stringer"),
        release_executable_path(workspace_root, "stringer-mcp"),
    ]
}

fn release_executable_path(workspace_root: impl AsRef<Path>, name: &str) -> PathBuf {
    workspace_root
        .as_ref()
        .join("target")
        .join("release")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

pub fn copy_release_binary_to_path_append_out_path(
    binary_path: impl AsRef<Path>,
    path_append_out_path: Option<PathBuf>,
) -> io::Result<PathAppendCopyOutcome> {
    let Some(out_dir) = path_append_out_path else {
        return Ok(PathAppendCopyOutcome::SkippedMissingEnv);
    };

    if !out_dir.is_dir() {
        return Ok(PathAppendCopyOutcome::SkippedMissingDirectory(out_dir));
    }

    let file_name = binary_path
        .as_ref()
        .file_name()
        .ok_or_else(|| io::Error::other("release binary path has no file name"))?;
    let destination = out_dir.join(file_name);
    fs::copy(binary_path, &destination)?;

    Ok(PathAppendCopyOutcome::Copied(destination))
}
