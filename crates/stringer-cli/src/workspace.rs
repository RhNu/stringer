use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use stringer_app::{
    InspectDiagnosticSeverityInput, InspectEntryStatusInput, SettingsInput,
    WorkspaceBatchApplyEntry, WorkspaceBatchApplyRequest, WorkspaceBatchClaimRequest,
    WorkspaceBatchCountRequest, WorkspaceBatchReleaseRequest, WorkspaceFinalizeRequest,
    WorkspaceInspectBatchRequest, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectEntriesRequest, WorkspaceInspectEntryRequest, WorkspaceInspectFilesRequest,
    WorkspaceOpenRequest, workspace_batch_apply, workspace_batch_claim, workspace_batch_count,
    workspace_batch_release, workspace_finalize, workspace_inspect_batch,
    workspace_inspect_diagnostics, workspace_inspect_entries, workspace_inspect_entry,
    workspace_inspect_files, workspace_open, workspace_upgrade_unsupported,
};

use crate::app::{CliError, print_json, read_input};
use crate::feedback::Feedback;
use crate::help::*;

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    #[command(
        about = "Open a translation workspace from a source root",
        long_about = WORKSPACE_OPEN_LONG_ABOUT,
        after_long_help = WORKSPACE_OPEN_AFTER_LONG_HELP
    )]
    Open(WorkspaceOpenCommand),
    #[command(
        about = "Finalize a translation workspace into an output directory",
        long_about = WORKSPACE_FINALIZE_LONG_ABOUT,
        after_long_help = WORKSPACE_FINALIZE_AFTER_LONG_HELP
    )]
    Finalize(WorkspaceFinalizeCommand),
    #[command(
        about = "Count, claim, apply, and release agent translation batches",
        long_about = WORKSPACE_BATCH_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Batch {
        #[command(subcommand)]
        command: WorkspaceBatchCommand,
    },
    #[command(
        about = "Read workspace files, entries, batches, and diagnostics without editing",
        arg_required_else_help = true
    )]
    Inspect {
        #[command(subcommand)]
        command: WorkspaceInspectCommand,
    },
    #[command(
        about = "Upgrade a legacy workspace to the current schema",
        long_about = WORKSPACE_UPGRADE_LONG_ABOUT,
        after_long_help = WORKSPACE_UPGRADE_AFTER_LONG_HELP
    )]
    Upgrade(WorkspaceUpgradeCommand),
}

#[derive(Debug, Parser)]
pub struct WorkspaceOpenCommand {
    #[arg(
        long,
        value_name = "SOURCE_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory to scan."
    )]
    pub source_root: Utf8PathBuf,
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory; defaults to the current directory."
    )]
    pub workspace: Utf8PathBuf,
    #[arg(long, help = "Replace generated workspace artifacts if present")]
    pub force: bool,
    #[arg(
        long,
        value_name = "GAME",
        help = "Game release, for example SkyrimSe",
        long_help = SETTINGS_LONG_HELP
    )]
    pub game_release: Option<String>,
    #[arg(
        long,
        value_name = "LANGUAGE",
        help = "Bethesda asset language, for example English",
        long_help = SETTINGS_LONG_HELP
    )]
    pub asset_language: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Source locale, for example en",
        long_help = SETTINGS_LONG_HELP
    )]
    pub source_locale: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Target locale, for example zh-Hans",
        long_help = SETTINGS_LONG_HELP
    )]
    pub target_locale: Option<String>,
}

#[derive(Debug, Parser)]
pub struct WorkspaceFinalizeCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory with workspace.json and entries/**/*.jsonl."
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "SOURCE_ROOT",
        help = "Optional source mod root override",
        long_help = "Optional source mod root override; defaults to source_root stored in workspace.json."
    )]
    pub source_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "OUTPUT",
        help = "Output directory",
        long_help = "Output directory outside the source root; defaults to <workspace>/output."
    )]
    pub output: Option<Utf8PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceBatchCommand {
    #[command(
        about = "Count translation work in a workspace or entry file",
        long_about = WORKSPACE_BATCH_COUNT_LONG_ABOUT
    )]
    Count(WorkspaceBatchCountCommand),
    #[command(
        about = "Claim a translation batch for agent work",
        long_about = WORKSPACE_BATCH_CLAIM_LONG_ABOUT
    )]
    Claim(WorkspaceBatchClaimCommand),
    #[command(
        about = "Apply translated entries for a claimed batch",
        long_about = WORKSPACE_BATCH_APPLY_LONG_ABOUT
    )]
    Apply(WorkspaceBatchApplyCommand),
    #[command(
        about = "Release a claimed translation batch",
        long_about = WORKSPACE_BATCH_RELEASE_LONG_ABOUT
    )]
    Release(WorkspaceBatchReleaseCommand),
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceInspectCommand {
    #[command(about = "List workspace entry files as JSON")]
    Files(WorkspaceInspectFilesCommand),
    #[command(about = "List workspace entries as JSON")]
    Entries(WorkspaceInspectEntriesCommand),
    #[command(about = "Read one workspace entry by id as JSON")]
    Entry(WorkspaceInspectEntryCommand),
    #[command(about = "Read a claimed batch as JSON")]
    Batch(WorkspaceInspectBatchCommand),
    #[command(about = "List workspace diagnostics as JSON")]
    Diagnostics(WorkspaceInspectDiagnosticsCommand),
}

#[derive(Debug, Parser)]
pub struct WorkspaceInspectFilesCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct WorkspaceInspectEntriesCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "ENTRY_FILE")]
    pub file: Option<String>,
    #[arg(long, default_value = "all", value_name = "STATUS")]
    pub status: InspectEntryStatusArg,
    #[arg(long, default_value_t = 50, value_name = "N")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub offset: usize,
}

#[derive(Debug, Parser)]
pub struct WorkspaceInspectEntryCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "ENTRY_ID")]
    pub id: String,
}

#[derive(Debug, Parser)]
pub struct WorkspaceInspectBatchCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "BATCH_ID")]
    pub batch_id: String,
    #[arg(long, default_value_t = 10, value_name = "N")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub offset: usize,
}

#[derive(Debug, Parser)]
pub struct WorkspaceInspectDiagnosticsCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "ENTRY_FILE")]
    pub file: Option<String>,
    #[arg(long, default_value = "all", value_name = "SEVERITY")]
    pub severity: InspectDiagnosticSeverityArg,
    #[arg(long, default_value_t = 50, value_name = "N")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum InspectEntryStatusArg {
    #[default]
    All,
    Empty,
    Memory,
    Translated,
    Claimed,
    Diagnostic,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum InspectDiagnosticSeverityArg {
    #[default]
    All,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchCountCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "ENTRY_FILE",
        help = "Optional workspace entry file listed in workspace.json"
    )]
    pub file: Option<String>,
    #[arg(long, help = "Emit structured JSON")]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchClaimCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "ENTRY_FILE",
        help = "Optional workspace entry file listed in workspace.json"
    )]
    pub file: Option<String>,
    #[arg(
        long,
        default_value_t = 50,
        value_name = "N",
        help = "Maximum entries to claim"
    )]
    pub limit: usize,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchApplyCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "PATCH_JSON",
        help = "Patch JSON file path, or - to read JSON from stdin"
    )]
    pub input: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchReleaseCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "BATCH_ID", help = "Batch id to release")]
    pub batch_id: String,
}

#[derive(Debug, Parser)]
pub struct WorkspaceUpgradeCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Legacy translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
}

pub async fn run_workspace(command: WorkspaceCommand, feedback: &Feedback) -> Result<(), CliError> {
    match command {
        WorkspaceCommand::Open(command) => {
            let status = feedback.command("workspace open");
            let summary = workspace_open(WorkspaceOpenRequest {
                source_root: command.source_root.to_string(),
                workspace: Some(command.workspace.to_string()),
                force: command.force,
                settings: settings_input(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                ),
            })
            .await?;
            status.finish();
            println!("opened workspace with {} entries", summary.entries);
            Ok(())
        }
        WorkspaceCommand::Finalize(command) => {
            let status = feedback.command("workspace finalize");
            let summary = workspace_finalize(WorkspaceFinalizeRequest {
                workspace: Some(command.workspace.to_string()),
                source_root: command.source_root.map(|path| path.to_string()),
                output: command.output.map(|path| path.to_string()),
            })
            .await?;
            status.finish();
            println!(
                "finalized workspace by applying {} entries and writing {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
        WorkspaceCommand::Batch { command } => run_workspace_batch(command, feedback),
        WorkspaceCommand::Inspect { command } => run_workspace_inspect(command, feedback),
        WorkspaceCommand::Upgrade(command) => {
            Err(workspace_upgrade_unsupported(command.workspace.to_string()).into())
        }
    }
}

fn run_workspace_inspect(
    command: WorkspaceInspectCommand,
    feedback: &Feedback,
) -> Result<(), CliError> {
    match command {
        WorkspaceInspectCommand::Files(command) => {
            let status = feedback.command("workspace inspect files");
            let inspected = workspace_inspect_files(WorkspaceInspectFilesRequest {
                workspace: Some(command.workspace.to_string()),
            })?;
            status.finish();
            print_json(&inspected)
        }
        WorkspaceInspectCommand::Entries(command) => {
            let status = feedback.command("workspace inspect entries");
            let inspected = workspace_inspect_entries(WorkspaceInspectEntriesRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file,
                status: command.status.into(),
                limit: command.limit,
                offset: command.offset,
            })?;
            status.finish();
            print_json(&inspected)
        }
        WorkspaceInspectCommand::Entry(command) => {
            let status = feedback.command("workspace inspect entry");
            let inspected = workspace_inspect_entry(WorkspaceInspectEntryRequest {
                workspace: Some(command.workspace.to_string()),
                id: command.id,
            })?;
            status.finish();
            print_json(&inspected)
        }
        WorkspaceInspectCommand::Batch(command) => {
            let status = feedback.command("workspace inspect batch");
            let inspected = workspace_inspect_batch(WorkspaceInspectBatchRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: command.batch_id,
                offset: command.offset,
                limit: command.limit,
            })?;
            status.finish();
            print_json(&inspected)
        }
        WorkspaceInspectCommand::Diagnostics(command) => {
            let status = feedback.command("workspace inspect diagnostics");
            let inspected = workspace_inspect_diagnostics(WorkspaceInspectDiagnosticsRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file,
                severity: command.severity.into(),
                limit: command.limit,
                offset: command.offset,
            })?;
            status.finish();
            print_json(&inspected)
        }
    }
}

fn run_workspace_batch(
    command: WorkspaceBatchCommand,
    feedback: &Feedback,
) -> Result<(), CliError> {
    match command {
        WorkspaceBatchCommand::Count(command) => {
            let status = feedback.command("workspace batch count");
            let count = workspace_batch_count(WorkspaceBatchCountRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file,
            })?;
            status.finish();
            if command.json {
                print_json(&count)?;
            } else {
                println!(
                    "counted {} entries: {} empty, {} memory-prefilled, {} translated, {} claimed, {} with diagnostics",
                    count.total,
                    count.empty,
                    count.memory_prefilled,
                    count.translated,
                    count.claimed,
                    count.diagnostics
                );
            }
            Ok(())
        }
        WorkspaceBatchCommand::Claim(command) => {
            let status = feedback.command("workspace batch claim");
            let claim = workspace_batch_claim(WorkspaceBatchClaimRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file,
                limit: command.limit,
            })?;
            status.finish();
            print_json(&claim)
        }
        WorkspaceBatchCommand::Apply(command) => {
            let status = feedback.command("workspace batch apply");
            let input = read_input(&command.input)?;
            let patch: WorkspaceBatchApplyPatchInput =
                serde_json::from_str(&input).map_err(|source| CliError::Json {
                    path: command.input.clone(),
                    source,
                })?;
            let summary = workspace_batch_apply(WorkspaceBatchApplyRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: patch.batch_id,
                entries: patch
                    .entries
                    .into_iter()
                    .map(|entry| WorkspaceBatchApplyEntry {
                        id: entry.id,
                        translation: entry.translation,
                    })
                    .collect(),
            })?;
            status.finish();
            print_json(&summary)
        }
        WorkspaceBatchCommand::Release(command) => {
            let status = feedback.command("workspace batch release");
            let summary = workspace_batch_release(WorkspaceBatchReleaseRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: command.batch_id,
            })?;
            status.finish();
            print_json(&summary)
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct WorkspaceBatchApplyPatchInput {
    batch_id: String,
    entries: Vec<WorkspaceBatchApplyPatchEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct WorkspaceBatchApplyPatchEntry {
    id: String,
    #[serde(default)]
    translation: Option<String>,
}

fn settings_input(
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
) -> SettingsInput {
    SettingsInput {
        game_release,
        asset_language,
        source_locale,
        target_locale,
    }
}

impl From<InspectEntryStatusArg> for InspectEntryStatusInput {
    fn from(value: InspectEntryStatusArg) -> Self {
        match value {
            InspectEntryStatusArg::All => Self::All,
            InspectEntryStatusArg::Empty => Self::Empty,
            InspectEntryStatusArg::Memory => Self::Memory,
            InspectEntryStatusArg::Translated => Self::Translated,
            InspectEntryStatusArg::Claimed => Self::Claimed,
            InspectEntryStatusArg::Diagnostic => Self::Diagnostic,
        }
    }
}

impl From<InspectDiagnosticSeverityArg> for InspectDiagnosticSeverityInput {
    fn from(value: InspectDiagnosticSeverityArg) -> Self {
        match value {
            InspectDiagnosticSeverityArg::All => Self::All,
            InspectDiagnosticSeverityArg::Error => Self::Error,
            InspectDiagnosticSeverityArg::Warning => Self::Warning,
            InspectDiagnosticSeverityArg::Info => Self::Info,
        }
    }
}

impl std::fmt::Display for InspectEntryStatusArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::All => "all",
            Self::Empty => "empty",
            Self::Memory => "memory",
            Self::Translated => "translated",
            Self::Claimed => "claimed",
            Self::Diagnostic => "diagnostic",
        })
    }
}

impl std::fmt::Display for InspectDiagnosticSeverityArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::All => "all",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        })
    }
}
