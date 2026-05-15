use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use stringer_app::{
    SettingsInput, WorkspaceBatchApplyEntry, WorkspaceBatchApplyRequest,
    WorkspaceBatchClaimRequest, WorkspaceBatchCountRequest, WorkspaceBatchReleaseRequest,
    WorkspaceFinalizeRequest, WorkspaceOpenRequest, workspace_batch_apply, workspace_batch_claim,
    workspace_batch_count, workspace_batch_release, workspace_finalize, workspace_open,
    workspace_upgrade_unsupported,
};

use crate::app::{CliError, print_json, read_input};
use crate::help::*;

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    #[command(
        about = "Open a translation workspace from a mod root",
        long_about = WORKSPACE_OPEN_LONG_ABOUT,
        after_long_help = WORKSPACE_OPEN_AFTER_LONG_HELP
    )]
    Open(WorkspaceOpenCommand),
    #[command(
        about = "Finalize a translation workspace into an override directory",
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
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. Stringer recursively reads plugin, STRINGS, PEX, and Scaleform translation-table assets from this directory."
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "WORKSPACE",
        help = "Translation workspace output directory",
        long_help = "Translation workspace output directory. The command creates workspace.json, batches/, and entries/**/*.jsonl. If the directory already exists, files managed by the workspace are rewritten with the current output."
    )]
    pub workspace: Utf8PathBuf,
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
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. finalize rereads the original assets from this directory before applying translation fields to matching entries."
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory. It must contain workspace.json and entries/**/*.jsonl. finalize only reads id and translation from each row."
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "OVERRIDE_ROOT",
        help = "Override output directory",
        long_help = "Override output directory. Stringer writes only changed assets and requires this directory to be outside the source mod root."
    )]
    pub override_root: Utf8PathBuf,
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

#[derive(Debug, Parser)]
pub struct WorkspaceBatchCountCommand {
    #[arg(
        long,
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
        value_name = "WORKSPACE",
        help = "Legacy translation workspace directory"
    )]
    pub workspace: Utf8PathBuf,
}

pub async fn run_workspace(command: WorkspaceCommand) -> Result<(), CliError> {
    match command {
        WorkspaceCommand::Open(command) => {
            let summary = workspace_open(WorkspaceOpenRequest {
                root: command.root.to_string(),
                workspace: command.workspace.to_string(),
                settings: settings_input(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                ),
            })
            .await?;
            println!("opened workspace with {} entries", summary.entries);
            Ok(())
        }
        WorkspaceCommand::Finalize(command) => {
            let summary = workspace_finalize(WorkspaceFinalizeRequest {
                root: command.root.to_string(),
                workspace: command.workspace.to_string(),
                override_root: command.override_root.to_string(),
            })
            .await?;
            println!(
                "finalized workspace by applying {} entries and writing {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
        WorkspaceCommand::Batch { command } => run_workspace_batch(command),
        WorkspaceCommand::Upgrade(command) => {
            Err(workspace_upgrade_unsupported(command.workspace.to_string()).into())
        }
    }
}

fn run_workspace_batch(command: WorkspaceBatchCommand) -> Result<(), CliError> {
    match command {
        WorkspaceBatchCommand::Count(command) => {
            let count = workspace_batch_count(WorkspaceBatchCountRequest {
                workspace: command.workspace.to_string(),
                file: command.file,
            })?;
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
            let claim = workspace_batch_claim(WorkspaceBatchClaimRequest {
                workspace: command.workspace.to_string(),
                file: command.file,
                limit: command.limit,
            })?;
            print_json(&claim)
        }
        WorkspaceBatchCommand::Apply(command) => {
            let input = read_input(&command.input)?;
            let patch: WorkspaceBatchApplyPatchInput =
                serde_json::from_str(&input).map_err(|source| CliError::Json {
                    path: command.input.clone(),
                    source,
                })?;
            let summary = workspace_batch_apply(WorkspaceBatchApplyRequest {
                workspace: command.workspace.to_string(),
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
            print_json(&summary)
        }
        WorkspaceBatchCommand::Release(command) => {
            let summary = workspace_batch_release(WorkspaceBatchReleaseRequest {
                workspace: command.workspace.to_string(),
                batch_id: command.batch_id,
            })?;
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
