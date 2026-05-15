use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use clap::{Args, Parser, Subcommand, error::ErrorKind};

use crate::{
    PathAppendCopyOutcome, copy_release_binary_to_path_append_out_path,
    find_line_budget_violations, release_binary_paths, release_build_args,
};

const DEFAULT_MAX_LINES: usize = 850;

#[derive(Debug, Parser)]
#[command(about = "Stringer workspace automation")]
struct Xtask {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Debug, Subcommand)]
enum XtaskCommand {
    #[command(about = "Check Rust source files under crates/ against a line budget")]
    LineBudget(LineBudgetArgs),
    #[command(
        about = "Build the CLI in release mode and optionally copy it to PATH_APPEND_OUT_PATH"
    )]
    Release,
}

#[derive(Debug, Args)]
struct LineBudgetArgs {
    #[arg(long, default_value_t = DEFAULT_MAX_LINES)]
    max_lines: usize,
}

pub fn run_from_env() -> Result<(), String> {
    run(env::args_os())
}

fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let xtask = match Xtask::try_parse_from(args) {
        Ok(xtask) => xtask,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            print!("{error}");
            return Ok(());
        }
        Err(error) => return Err(error.to_string()),
    };

    match xtask.command {
        XtaskCommand::LineBudget(args) => run_line_budget(args),
        XtaskCommand::Release => run_release(),
    }
}

fn run_line_budget(args: LineBudgetArgs) -> Result<(), String> {
    let workspace_root = env::current_dir().map_err(|error| error.to_string())?;
    let violations = find_line_budget_violations(&workspace_root, args.max_lines)
        .map_err(|error| error.to_string())?;

    if violations.is_empty() {
        println!(
            "All crates/**/*.rs files are at or below {} lines.",
            args.max_lines
        );
        return Ok(());
    }

    eprintln!(
        "The following crates/**/*.rs files exceed {} lines:",
        args.max_lines
    );
    for violation in violations {
        eprintln!(
            "- {}: {} lines",
            violation.path.display(),
            violation.line_count
        );
    }
    Err("line budget check failed".to_owned())
}

fn run_release() -> Result<(), String> {
    let workspace_root = env::current_dir().map_err(|error| error.to_string())?;
    let status = ProcessCommand::new("cargo")
        .args(release_build_args())
        .status()
        .map_err(|error| format!("failed to run cargo build: {error}"))?;

    if !status.success() {
        return Err(format!("cargo build failed with {status}"));
    }

    let path_append_out_path = env::var_os("PATH_APPEND_OUT_PATH").map(PathBuf::from);
    for binary_path in release_binary_paths(&workspace_root) {
        match copy_release_binary_to_path_append_out_path(binary_path, path_append_out_path.clone())
            .map_err(|error| format!("failed to copy release binary: {error}"))?
        {
            PathAppendCopyOutcome::Copied(destination) => {
                println!("Copied release binary to {}.", destination.display());
            }
            PathAppendCopyOutcome::SkippedMissingEnv => {
                println!("PATH_APPEND_OUT_PATH is not defined; skipped release binary copy.");
                break;
            }
            PathAppendCopyOutcome::SkippedMissingDirectory(path) => {
                println!(
                    "PATH_APPEND_OUT_PATH does not point to an existing directory ({}); skipped release binary copy.",
                    path.display()
                );
                break;
            }
        }
    }

    Ok(())
}
