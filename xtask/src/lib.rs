use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Eq, PartialEq)]
pub struct LineBudgetViolation {
    pub path: PathBuf,
    pub line_count: usize,
}

pub fn find_line_budget_violations(
    workspace_root: impl AsRef<Path>,
    max_lines: usize,
) -> io::Result<Vec<LineBudgetViolation>> {
    let mut violations = Vec::new();
    let crates_dir = workspace_root.as_ref().join("crates");

    if !crates_dir.exists() {
        return Ok(violations);
    }

    for entry in WalkDir::new(crates_dir) {
        let entry = entry.map_err(io::Error::other)?;
        let path = entry.path();

        if !entry.file_type().is_file()
            || path.extension().is_none_or(|extension| extension != "rs")
        {
            continue;
        }

        let line_count = count_lines(path)?;
        if line_count > max_lines {
            violations.push(LineBudgetViolation {
                path: path.to_path_buf(),
                line_count,
            });
        }
    }

    violations.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(violations)
}

fn count_lines(path: &Path) -> io::Result<usize> {
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Ok(0);
    }

    let newline_count = bytes.iter().filter(|byte| **byte == b'\n').count();
    Ok(newline_count + usize::from(!bytes.ends_with(b"\n")))
}
