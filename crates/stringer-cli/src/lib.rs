#![forbid(unsafe_code)]

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use stringer_workspace::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, ExportTranslationsOptions,
    ImportTranslationsOptions, KnowledgeLayerOverrides, LoadWorkspaceSettingsOptions,
    LookupKnowledgeOptions, PipelineEntryKind, ValidateTranslationsOptions, WorkspaceError,
    WorkspaceSettingsOverrides, WriteTarget, annotate_translations, build_knowledge_index,
    export_translations, import_translations, load_workspace_settings, lookup_knowledge,
    parse_game_release_name, parse_language_name, validate_translations,
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
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
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
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeCommand {
    Annotate(KnowledgeAnnotateCommand),
    Validate(KnowledgeValidateCommand),
    Lookup(KnowledgeLookupCommand),
    Index {
        #[command(subcommand)]
        command: KnowledgeIndexCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeIndexCommand {
    Rebuild(KnowledgeIndexRebuildCommand),
}

#[derive(Debug, Parser)]
pub struct KnowledgeAnnotateCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub translations: Utf8PathBuf,
    #[arg(long)]
    pub auto_fill_memory: bool,
    #[arg(long)]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(long)]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeValidateCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub translations: Utf8PathBuf,
    #[arg(long)]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(long)]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeLookupCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub text: String,
    #[arg(long, default_value = "plugin")]
    pub kind: String,
    #[arg(long)]
    pub record_type: Option<String>,
    #[arg(long)]
    pub subrecord: Option<String>,
    #[arg(long)]
    pub game_release: Option<String>,
    #[arg(long)]
    pub asset_language: Option<String>,
    #[arg(long)]
    pub source_locale: Option<String>,
    #[arg(long)]
    pub target_locale: Option<String>,
    #[arg(long)]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(long)]
    pub override_knowledge_root: Option<Utf8PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeIndexRebuildCommand {
    #[arg(long)]
    pub root: Utf8PathBuf,
    #[arg(long)]
    pub game_release: Option<String>,
    #[arg(long)]
    pub asset_language: Option<String>,
    #[arg(long)]
    pub source_locale: Option<String>,
    #[arg(long)]
    pub target_locale: Option<String>,
    #[arg(long)]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(long)]
    pub override_knowledge_root: Option<Utf8PathBuf>,
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
            let summary = export_translations(ExportTranslationsOptions {
                root: command.root,
                out: command.out,
                settings,
            })
            .await?;
            println!("exported {} entries", summary.entries);
            Ok(())
        }
        Command::Import(command) => {
            let summary = import_translations(ImportTranslationsOptions {
                root: command.root,
                translations: command.translations,
                target: WriteTarget::OverrideDirectory {
                    root: command.override_root,
                },
            })
            .await?;
            println!(
                "applied {} entries and wrote {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
        Command::Knowledge { command } => run_knowledge(command).await,
    }
}

async fn run_knowledge(command: KnowledgeCommand) -> Result<(), WorkspaceError> {
    match command {
        KnowledgeCommand::Annotate(command) => {
            let summary = annotate_translations(AnnotateTranslationsOptions {
                root: command.root,
                translations: command.translations,
                allow_memory_auto_fill: command.auto_fill_memory,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "annotated {} entries, added {} annotations, wrote {} diagnostics, auto-filled {} entries",
                summary.entries, summary.annotations, summary.diagnostics, summary.auto_filled
            );
            Ok(())
        }
        KnowledgeCommand::Validate(command) => {
            let summary = validate_translations(ValidateTranslationsOptions {
                root: command.root,
                translations: command.translations,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "validated {} entries and wrote {} diagnostics",
                summary.entries, summary.diagnostics
            );
            Ok(())
        }
        KnowledgeCommand::Lookup(command) => {
            let config_path = project_config_path(&command.root);
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let lookup = lookup_knowledge(LookupKnowledgeOptions {
                root: command.root,
                settings,
                text: command.text,
                kind: parse_pipeline_kind(command.kind)?,
                context: lookup_context(command.record_type, command.subrecord),
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            if command.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "index_used": lookup.index_used,
                        "annotations": lookup.annotations,
                        "diagnostics": lookup.diagnostics,
                    }))
                    .map_err(|source| WorkspaceError::Json {
                        path: Utf8PathBuf::from("<stdout>"),
                        source,
                    })?
                );
            } else {
                println!(
                    "found {} annotations and {} diagnostics",
                    lookup.annotations.len(),
                    lookup.diagnostics.len()
                );
            }
            Ok(())
        }
        KnowledgeCommand::Index { command } => run_knowledge_index(command).await,
    }
}

async fn run_knowledge_index(command: KnowledgeIndexCommand) -> Result<(), WorkspaceError> {
    match command {
        KnowledgeIndexCommand::Rebuild(command) => {
            let config_path = project_config_path(&command.root);
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
                root: command.root,
                settings,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "indexed {} files, {} terms, {} memory entries, {} rules, {} diagnostics",
                summary.files, summary.terms, summary.memory, summary.rules, summary.diagnostics
            );
            Ok(())
        }
    }
}

fn parse_pipeline_kind(value: String) -> Result<PipelineEntryKind, WorkspaceError> {
    PipelineEntryKind::from_package_kind(&value).ok_or(WorkspaceError::InvalidSetting {
        name: "kind",
        value,
    })
}

fn lookup_context(record_type: Option<String>, subrecord: Option<String>) -> Vec<(String, String)> {
    let mut context = Vec::new();
    if let Some(record_type) = record_type {
        context.push(("record_type".to_string(), record_type));
    }
    if let Some(subrecord) = subrecord {
        context.push(("subrecord".to_string(), subrecord));
    }
    context
}

fn knowledge_overrides(
    global_root: Option<Utf8PathBuf>,
    override_root: Option<Utf8PathBuf>,
) -> KnowledgeLayerOverrides {
    KnowledgeLayerOverrides {
        global_root,
        override_root,
    }
}

fn project_config_path(root: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    let path = root.join("stringer.toml");
    path.exists().then_some(path)
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
