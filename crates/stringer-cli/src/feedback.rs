use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ProgressModeArg {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Feedback {
    enabled: bool,
}

impl Feedback {
    pub(crate) fn new(quiet: bool, progress: ProgressModeArg) -> Self {
        let enabled = !quiet
            && match progress {
                ProgressModeArg::Auto => io::stderr().is_terminal(),
                ProgressModeArg::Always => true,
                ProgressModeArg::Never => false,
            };
        Self { enabled }
    }

    pub(crate) fn command(&self, message: impl Into<String>) -> CommandStatus {
        let message = message.into();
        let progress = self.enabled.then(|| {
            let progress = ProgressBar::new_spinner();
            progress.set_style(spinner_style());
            progress.set_message(message.clone());
            progress.enable_steady_tick(Duration::from_millis(120));
            progress
        });
        CommandStatus {
            progress,
            message,
            started: Instant::now(),
        }
    }

    pub(crate) fn progress(&self, message: impl Into<String>, total: u64) -> ProgressHandle {
        let message = message.into();
        let progress = self.enabled.then(|| {
            let progress = ProgressBar::new(total);
            progress.set_style(bar_style());
            progress.set_message(message.clone());
            progress
        });
        ProgressHandle {
            progress,
            message,
            started: Instant::now(),
        }
    }
}

pub(crate) struct CommandStatus {
    progress: Option<ProgressBar>,
    message: String,
    started: Instant,
}

impl CommandStatus {
    pub(crate) fn finish(mut self) {
        if let Some(progress) = self.progress.take() {
            progress.finish_and_clear();
            eprintln!("done: {} in {}", self.message, elapsed(self.started));
        }
    }
}

impl Drop for CommandStatus {
    fn drop(&mut self) {
        if let Some(progress) = self.progress.take() {
            progress.finish_and_clear();
        }
    }
}

pub(crate) struct ProgressHandle {
    progress: Option<ProgressBar>,
    message: String,
    started: Instant,
}

impl ProgressHandle {
    pub(crate) fn set_position(&self, position: u64) {
        if let Some(progress) = &self.progress {
            progress.set_position(position);
        }
    }

    pub(crate) fn set_message(&mut self, message: impl Into<String>) {
        if let Some(progress) = &self.progress {
            progress.set_message(message.into());
        }
    }

    pub(crate) fn finish(mut self) {
        if let Some(progress) = self.progress.take() {
            progress.finish_and_clear();
            eprintln!("done: {} in {}", self.message, elapsed(self.started));
        }
    }
}

impl Drop for ProgressHandle {
    fn drop(&mut self) {
        if let Some(progress) = self.progress.take() {
            progress.finish_and_clear();
        }
    }
}

pub(crate) fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(default_filter(verbose)))
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal())
        .try_init();
}

fn default_filter(verbose: u8) -> &'static str {
    match verbose {
        0 => "warn",
        1 => {
            "warn,stringer_cli=debug,stringer_app=debug,stringer_knowledge=debug,stringer_workspace_api=debug,stringer_workspace_ops=debug,stringer_adapt=debug,stringer_reader=debug,stringer_plugin=debug,stringer_pex=debug,stringer_scaleform=debug,stringer_core=debug"
        }
        _ => {
            "warn,stringer_cli=trace,stringer_app=trace,stringer_knowledge=trace,stringer_workspace_api=trace,stringer_workspace_ops=trace,stringer_adapt=trace,stringer_reader=trace,stringer_plugin=trace,stringer_pex=trace,stringer_scaleform=trace,stringer_core=trace"
        }
    }
}

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.green} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template("{bar:40.cyan/blue} {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("=>-")
}

fn elapsed(started: Instant) -> String {
    let elapsed = started.elapsed();
    if elapsed.as_secs() > 0 {
        format!("{:.1}s", elapsed.as_secs_f64())
    } else {
        format!("{}ms", elapsed.as_millis())
    }
}
