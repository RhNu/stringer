#![forbid(unsafe_code)]

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use stringer_workspace::{
    ExportTranslationsOptions, ImportTranslationsOptions, LoadWorkspaceSettingsOptions,
    WorkspaceError, WorkspaceSettingsOverrides, WriteTarget, export_translation_jsonl,
    import_translation_jsonl, load_workspace_settings, parse_game_release_name,
    parse_language_name,
};

#[derive(Debug, Parser)]
#[command(name = "stringer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Export(ExportCommand),
    Import(ImportCommand),
}

#[derive(Debug, Parser)]
pub struct ExportCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub out: Utf8PathBuf,
    #[arg(long)]
    pub game_release: Option<String>,
    #[arg(long)]
    pub asset_language: Option<String>,
    #[arg(long)]
    pub source_locale: Option<String>,
    #[arg(long)]
    pub target_locale: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ImportCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub translations: Utf8PathBuf,
    #[arg(long)]
    pub override_root: Utf8PathBuf,
    #[arg(long)]
    pub game_release: Option<String>,
    #[arg(long)]
    pub asset_language: Option<String>,
    #[arg(long)]
    pub source_locale: Option<String>,
    #[arg(long)]
    pub target_locale: Option<String>,
}

pub async fn run(cli: Cli) -> Result<(), WorkspaceError> {
    match cli.command {
        Command::Export(command) => {
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path: None,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = export_translation_jsonl(ExportTranslationsOptions {
                root: command.root,
                out: command.out,
                settings,
            })
            .await?;
            println!("exported {} entries", summary.entries);
            Ok(())
        }
        Command::Import(command) => {
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path: None,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = import_translation_jsonl(ImportTranslationsOptions {
                root: command.root,
                translations: command.translations,
                target: WriteTarget::OverrideDirectory {
                    root: command.override_root,
                },
                settings,
            })
            .await?;
            println!(
                "applied {} entries and wrote {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
    }
}

fn overrides(
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
) -> Result<WorkspaceSettingsOverrides, WorkspaceError> {
    Ok(WorkspaceSettingsOverrides {
        game_release: game_release
            .as_deref()
            .map(parse_game_release_name)
            .transpose()?,
        asset_language: asset_language
            .as_deref()
            .map(parse_language_name)
            .transpose()?,
        source_locale,
        target_locale,
    })
}
