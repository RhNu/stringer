use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use stringer_workspace::{
    ExportTranslationsOptions, ImportTranslationsOptions, LoadWorkspaceSettingsOptions,
    WorkspaceError, WriteTarget, export_translations, import_translations, load_workspace_settings,
};

use crate::cli_settings::overrides;
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
        long_help = "Translation workspace output directory. The command creates manifest.json and entries/**/*.jsonl. If the directory already exists, files managed by the workspace are rewritten with the current output."
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
        long_help = "Translation workspace directory. It must contain manifest.json and entries/**/*.jsonl. finalize only reads id and translation from each row."
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

pub async fn run_workspace(command: WorkspaceCommand) -> Result<(), WorkspaceError> {
    match command {
        WorkspaceCommand::Open(command) => {
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                user_config_path: None,
                project_config_path: project_config_path(&command.root),
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = export_translations(ExportTranslationsOptions {
                root: command.root,
                out: command.workspace,
                settings,
            })
            .await?;
            println!("opened workspace with {} entries", summary.entries);
            Ok(())
        }
        WorkspaceCommand::Finalize(command) => {
            let summary = import_translations(ImportTranslationsOptions {
                root: command.root,
                translations: command.workspace,
                target: WriteTarget::OverrideDirectory {
                    root: command.override_root,
                },
            })
            .await?;
            println!(
                "finalized workspace by applying {} entries and writing {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
    }
}

fn project_config_path(root: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    let path = root.join("stringer.toml");
    path.exists().then_some(path)
}
