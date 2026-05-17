use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use stringer_app::{
    workspace_batch_claim, workspace_batch_count, workspace_batch_detail, workspace_batch_export,
    workspace_batch_read, workspace_batch_release, workspace_finalize,
    workspace_inspect_diagnostics, workspace_inspect_entries, workspace_inspect_entry,
    workspace_inspect_files, workspace_normalize, workspace_open,
};
use stringer_interface::{
    InspectDiagnosticSeverityInput, InspectEntryStatusInput, SettingsInput,
    WorkspaceBatchClaimRequest, WorkspaceBatchCountRequest, WorkspaceBatchDetailRequest,
    WorkspaceBatchExportFormatInput, WorkspaceBatchExportRequest, WorkspaceBatchReadRequest,
    WorkspaceBatchReleaseRequest, WorkspaceFinalizeRequest, WorkspaceInspectDiagnosticsRequest,
    WorkspaceInspectEntriesRequest, WorkspaceInspectEntryRequest, WorkspaceInspectFilesRequest,
    WorkspaceNormalizeEncodingInput, WorkspaceNormalizeRequest, WorkspaceNormalizeResponse,
    WorkspaceOpenRequest,
};
use stringer_workspace_api::submit_batch;

use crate::app::{CliError, print_json};
use crate::feedback::Feedback;
use crate::help::*;
use crate::workspace_batch_input::read_batch_submit_input;

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
    #[command(about = "Normalize existing translations with xTranslator Search/Replace rules")]
    Normalize(WorkspaceNormalizeCommand),
    #[command(
        about = "Count, claim, submit, and release agent translation batches",
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
    #[arg(
        long,
        help = "Finalize even if the workspace has unfinished rows, claims, or diagnostics"
    )]
    pub force: bool,
}

#[derive(Debug, Parser)]
pub struct WorkspaceNormalizeCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "RULES_TXT",
        help = "xTranslator batch Search/Replace rules file"
    )]
    pub rules: Utf8PathBuf,
    #[arg(
        long,
        value_name = "ENTRY_FILE",
        help = "Optional workspace entry file listed in workspace.json"
    )]
    pub file: Option<String>,
    #[arg(long, help = "Write normalized translations back to the workspace")]
    pub apply: bool,
    #[arg(long, default_value = "auto", value_name = "ENCODING")]
    pub encoding: WorkspaceNormalizeEncodingArg,
    #[arg(long, default_value_t = 50, value_name = "N")]
    pub limit: usize,
    #[arg(long, help = "Emit structured JSON")]
    pub json: bool,
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
        about = "Read compact entries from a claimed batch",
        long_about = WORKSPACE_BATCH_READ_LONG_ABOUT
    )]
    Read(WorkspaceBatchReadCommand),
    #[command(
        about = "Read full detail for one or more batch entry keys",
        long_about = WORKSPACE_BATCH_DETAIL_LONG_ABOUT
    )]
    Detail(WorkspaceBatchDetailCommand),
    #[command(
        about = "Submit translated, skipped, or pending batch entries",
        long_about = WORKSPACE_BATCH_SUBMIT_LONG_ABOUT
    )]
    Submit(WorkspaceBatchSubmitCommand),
    #[command(
        about = "Export an editable batch submission file",
        long_about = WORKSPACE_BATCH_EXPORT_LONG_ABOUT
    )]
    Export(WorkspaceBatchExportCommand),
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
    Skipped,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum WorkspaceNormalizeEncodingArg {
    #[default]
    Auto,
    #[value(name = "utf-8")]
    Utf8,
    Cp936,
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
pub struct WorkspaceBatchReadCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "BATCH_ID", help = "Batch id to read")]
    pub batch_id: String,
    #[arg(long, default_value_t = 10, value_name = "N")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, value_name = "N")]
    pub offset: usize,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchDetailCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "BATCH_ID")]
    pub batch_id: String,
    #[arg(long = "key", value_name = "KEY", required = true)]
    pub keys: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchSubmitCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "SUBMISSION_JSON_OR_CSV",
        help = "Submission JSON/CSV file path, or - to read JSON from stdin"
    )]
    pub input: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct WorkspaceBatchExportCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "BATCH_ID")]
    pub batch_id: String,
    #[arg(long, value_name = "DIR")]
    pub out: Option<Utf8PathBuf>,
    #[arg(long, default_value = "json", value_name = "FORMAT")]
    pub format: WorkspaceBatchExportFormatArg,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum WorkspaceBatchExportFormatArg {
    #[default]
    Json,
    Csv,
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
                force: command.force,
            })
            .await?;
            status.finish();
            println!(
                "finalized workspace by applying {} entries and writing {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
        WorkspaceCommand::Normalize(command) => run_workspace_normalize(command, feedback),
        WorkspaceCommand::Batch { command } => run_workspace_batch(command, feedback),
        WorkspaceCommand::Inspect { command } => run_workspace_inspect(command, feedback),
    }
}

fn run_workspace_normalize(
    command: WorkspaceNormalizeCommand,
    feedback: &Feedback,
) -> Result<(), CliError> {
    let status = feedback.command("workspace normalize");
    let summary = workspace_normalize(WorkspaceNormalizeRequest {
        workspace: Some(command.workspace.to_string()),
        rules: command.rules.to_string(),
        file: command.file,
        apply: command.apply,
        encoding: command.encoding.into(),
        limit: command.limit,
    })?;
    status.finish();
    if command.json {
        return print_json(&summary);
    }
    print_normalize_summary(&summary, command.apply);
    Ok(())
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
                    "counted {} entries: {} claimable, {} empty, {} memory-prefilled, {} translated, {} skipped, {} claimed, {} with diagnostics",
                    count.total,
                    count.claimable,
                    count.empty,
                    count.memory_prefilled,
                    count.translated,
                    count.skipped,
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
        WorkspaceBatchCommand::Read(command) => {
            let status = feedback.command("workspace batch read");
            let page = workspace_batch_read(WorkspaceBatchReadRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: command.batch_id,
                offset: command.offset,
                limit: command.limit,
            })?;
            status.finish();
            print_json(&page)
        }
        WorkspaceBatchCommand::Detail(command) => {
            let status = feedback.command("workspace batch detail");
            let detail = workspace_batch_detail(WorkspaceBatchDetailRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: command.batch_id,
                keys: command.keys,
            })?;
            status.finish();
            print_json(&detail)
        }
        WorkspaceBatchCommand::Submit(command) => {
            let status = feedback.command("workspace batch submit");
            let submission = read_batch_submit_input(&command.workspace, &command.input)?;
            let summary = submit_batch(submission)?;
            status.finish();
            print_json(&summary)
        }
        WorkspaceBatchCommand::Export(command) => {
            let status = feedback.command("workspace batch export");
            let out = command.out.map(|dir| {
                dir.join(match command.format {
                    WorkspaceBatchExportFormatArg::Json => "patch.json",
                    WorkspaceBatchExportFormatArg::Csv => "patch.csv",
                })
            });
            let summary = workspace_batch_export(WorkspaceBatchExportRequest {
                workspace: Some(command.workspace.to_string()),
                batch_id: command.batch_id,
                out: out.map(|path| path.to_string()),
                format: command.format.into(),
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

fn print_normalize_summary(summary: &WorkspaceNormalizeResponse, apply: bool) {
    let action = if apply { "applied" } else { "dry-run" };
    println!(
        "{action} normalization: scanned {} translated entries, changed {}, replacements {}, skipped {} claimed and {} placeholder-risk entries",
        summary.scanned_entries,
        summary.changed_entries,
        summary.total_replacements,
        summary.skipped_claimed,
        summary.skipped_placeholder_risk
    );
    if !summary.warnings.is_empty() {
        println!("rule warnings: {}", summary.warnings.len());
    }
    if !apply {
        println!("rerun with --apply to write these changes");
    }
    println!(
        "run `stringer knowledge validate` after applying normalization to refresh diagnostics"
    );
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
            InspectEntryStatusArg::Skipped => Self::Skipped,
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

impl From<WorkspaceNormalizeEncodingArg> for WorkspaceNormalizeEncodingInput {
    fn from(value: WorkspaceNormalizeEncodingArg) -> Self {
        match value {
            WorkspaceNormalizeEncodingArg::Auto => Self::Auto,
            WorkspaceNormalizeEncodingArg::Utf8 => Self::Utf8,
            WorkspaceNormalizeEncodingArg::Cp936 => Self::Cp936,
        }
    }
}

impl From<WorkspaceBatchExportFormatArg> for WorkspaceBatchExportFormatInput {
    fn from(value: WorkspaceBatchExportFormatArg) -> Self {
        match value {
            WorkspaceBatchExportFormatArg::Json => Self::Json,
            WorkspaceBatchExportFormatArg::Csv => Self::Csv,
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
            Self::Skipped => "skipped",
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

impl std::fmt::Display for WorkspaceNormalizeEncodingArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Auto => "auto",
            Self::Utf8 => "utf-8",
            Self::Cp936 => "cp936",
        })
    }
}

impl std::fmt::Display for WorkspaceBatchExportFormatArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Json => "json",
            Self::Csv => "csv",
        })
    }
}
