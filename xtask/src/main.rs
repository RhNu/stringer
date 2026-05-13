use std::env;
use std::process::ExitCode;

use xtask::find_line_budget_violations;

const DEFAULT_MAX_LINES: usize = 850;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(usage());
    };

    match command.as_str() {
        "line-budget" => run_line_budget(args.collect()),
        _ => Err(usage()),
    }
}

fn run_line_budget(args: Vec<String>) -> Result<(), String> {
    let max_lines = parse_line_budget_args(args)?;
    let workspace_root = env::current_dir().map_err(|error| error.to_string())?;
    let violations = find_line_budget_violations(&workspace_root, max_lines)
        .map_err(|error| error.to_string())?;

    if violations.is_empty() {
        println!("All crates/**/*.rs files are at or below {max_lines} lines.");
        return Ok(());
    }

    eprintln!("The following crates/**/*.rs files exceed {max_lines} lines:");
    for violation in violations {
        eprintln!(
            "- {}: {} lines",
            violation.path.display(),
            violation.line_count
        );
    }
    Err("line budget check failed".to_owned())
}

fn parse_line_budget_args(args: Vec<String>) -> Result<usize, String> {
    match args.as_slice() {
        [] => Ok(DEFAULT_MAX_LINES),
        [flag, value] if flag == "--max-lines" => value
            .parse()
            .map_err(|_| "--max-lines must be a positive integer".to_owned()),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: cargo run -p xtask -- line-budget [--max-lines N]".to_owned()
}
