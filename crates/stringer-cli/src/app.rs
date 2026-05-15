use camino::Utf8PathBuf;
use clap::{Parser, Subcommand, ValueEnum};
use stringer_adapt::{
    AdaptError, AdaptFormat, AdaptImportOptions, merge_memory_jsonl, read_adapt_catalog,
    write_memory_jsonl,
};
use stringer_workspace::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeLayerOverrides,
    LoadWorkspaceSettingsOptions, LookupKnowledgeField, LookupKnowledgeMode,
    LookupKnowledgeOptions, LookupKnowledgeSource, PipelineEntryKind, ValidateTranslationsOptions,
    WorkspaceError, annotate_translations, build_knowledge_index, game_release_name,
    load_global_knowledge_root, load_workspace_settings, lookup_knowledge, parse_game_release_name,
    validate_translations,
};
use thiserror::Error;

use crate::cli_settings::overrides;
use crate::help::*;
use crate::workspace::{WorkspaceCommand, run_workspace};

#[derive(Debug, Parser)]
#[command(
    name = "stringer",
    version,
    about = "Bethesda mod localization workspace and knowledge tool",
    long_about = ROOT_LONG_ABOUT,
    after_long_help = ROOT_AFTER_LONG_HELP,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(
        about = "Open and finalize translation workspaces",
        long_about = WORKSPACE_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    #[command(
        about = "Adapt external translation resources into Stringer memory",
        long_about = ADAPT_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Adapt {
        #[command(subcommand)]
        command: AdaptCommand,
    },
    #[command(
        about = "Terminology, memory, rule, and diagnostic tools",
        long_about = KNOWLEDGE_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AdaptCommand {
    #[command(
        about = "Import an external translation resource as memory JSONL",
        long_about = ADAPT_IMPORT_LONG_ABOUT,
        after_long_help = ADAPT_IMPORT_AFTER_LONG_HELP
    )]
    Import(AdaptImportCommand),
}

#[derive(Debug, Parser)]
pub struct AdaptImportCommand {
    #[arg(
        long,
        value_name = "FORMAT",
        help = "Input format: eet, eet-xml, eet-json, or xt-sst",
        long_help = "Input format. Use eet for ESP-ESM Translator binary EET tables, eet-xml for EET XML exports, eet-json for EET JSON or DDS-style exports, and xt-sst for xTranslator SST files."
    )]
    pub format: AdaptFormatArg,
    #[arg(
        long,
        value_name = "INPUT",
        help = "External translation resource to read",
        long_help = "External translation resource to read. The parser selected by --format reads this file and extracts non-empty source/target pairs."
    )]
    pub input: Utf8PathBuf,
    #[arg(
        long,
        value_name = "MEMORY_JSONL",
        help = "Output Stringer memory JSONL path",
        long_help = "Output Stringer memory JSONL path. When omitted, adapt merges into the global user knowledge root under memory/adapt/<INPUT_FILE_NAME>.jsonl."
    )]
    pub out: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Source locale to write into memory rows",
        long_help = "Source locale to write into every generated memory row, for example en."
    )]
    pub source_locale: String,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Target locale to write into memory rows",
        long_help = "Target locale to write into every generated memory row, for example zh-Hans."
    )]
    pub target_locale: String,
    #[arg(
        long,
        value_name = "GAME",
        help = "Optional game context, for example SkyrimSe",
        long_help = "Optional game context. Accepted names follow the same normalization as --game-release, for example SkyrimSe or skyrim-se. When valid, the generated memory context includes game=SkyrimSe or game=SkyrimLe."
    )]
    pub game: Option<String>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Override the global user knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AdaptFormatArg {
    #[value(name = "eet")]
    Eet,
    #[value(name = "eet-xml")]
    EetXml,
    #[value(name = "eet-json")]
    EetJson,
    #[value(name = "xt-sst")]
    XtSst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KnowledgeLookupSourceArg {
    All,
    Memory,
    Terms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum KnowledgeLookupFieldArg {
    Both,
    Source,
    Target,
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeCommand {
    #[command(
        about = "Write knowledge hints into a translation package",
        long_about = ANNOTATE_LONG_ABOUT,
        after_long_help = ANNOTATE_AFTER_LONG_HELP
    )]
    Annotate(KnowledgeAnnotateCommand),
    #[command(
        about = "Validate a translation package and write diagnostics",
        long_about = VALIDATE_LONG_ABOUT,
        after_long_help = VALIDATE_AFTER_LONG_HELP
    )]
    Validate(KnowledgeValidateCommand),
    #[command(
        about = "Search terminology and memory tables",
        long_about = LOOKUP_LONG_ABOUT,
        after_long_help = LOOKUP_AFTER_LONG_HELP
    )]
    Lookup(KnowledgeLookupCommand),
    #[command(
        about = "Maintain the derived knowledge index",
        long_about = INDEX_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Index {
        #[command(subcommand)]
        command: KnowledgeIndexCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeIndexCommand {
    #[command(
        about = "Rebuild the knowledge SQLite index",
        long_about = INDEX_REBUILD_LONG_ABOUT,
        after_long_help = INDEX_REBUILD_AFTER_LONG_HELP
    )]
    Rebuild(KnowledgeIndexRebuildCommand),
}

#[derive(Debug, Parser)]
pub struct KnowledgeAnnotateCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. Used to locate project knowledge at <MOD_ROOT>/knowledge and, when present, read knowledge.global_root from <MOD_ROOT>/stringer.toml."
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "Translation package directory",
        long_help = "Translation package directory. annotate updates entries/**/*.jsonl in place, writing hints, diagnostics, and optionally filling translation."
    )]
    pub translations: Utf8PathBuf,
    #[arg(
        long,
        help = "Allow high-confidence memory to fill empty translations",
        long_help = "Allow high-confidence translation memory to fill translation. Disabled by default. When enabled, it only fills empty translations that meet the threshold and does not overwrite existing agent translations."
    )]
    pub auto_fill_memory: bool,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Override the global knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Add a highest-priority knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeValidateCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. Used to locate project knowledge at <MOD_ROOT>/knowledge and, when present, read knowledge.global_root from <MOD_ROOT>/stringer.toml."
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "Translation package directory",
        long_help = "Translation package directory. validate updates entries/**/*.jsonl in place and recomputes diagnostics for each row."
    )]
    pub translations: Utf8PathBuf,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Override the global knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Add a highest-priority knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeLookupCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. lookup uses it to locate project knowledge, the knowledge index, and optional stringer.toml."
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TEXT",
        help = "Text query to search in knowledge source and target fields",
        long_help = "Text query to search in knowledge source and target fields. By default lookup treats this as a case-insensitive contains query; pass --regex to treat it as a regex pattern."
    )]
    pub text: String,
    #[arg(
        long,
        default_value = "plugin",
        value_name = "KIND",
        help = "Entry kind: plugin, strings, scaleform, or pex",
        long_help = "Entry kind. Available values: plugin, strings, scaleform, pex. Knowledge enrichment primarily covers plugin, strings, and scaleform; pex is accepted to keep the interface uniform."
    )]
    pub kind: String,
    #[arg(
        long,
        value_name = "RECORD_TYPE",
        help = "Plugin record type, for example WEAP",
        long_help = "Plugin record type, for example WEAP, ARMO, or NPC_. Terminology scopes can use it for more precise matching."
    )]
    pub record_type: Option<String>,
    #[arg(
        long,
        value_name = "SUBRECORD",
        help = "Plugin subrecord, for example FULL",
        long_help = "Plugin subrecord, for example FULL or DESC. Terminology and translation memory can use it to constrain context."
    )]
    pub subrecord: Option<String>,
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
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Override the global knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Add a highest-priority knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        help = "Treat --text as a regex pattern instead of a contains query",
        long_help = "Treat --text as a regex pattern instead of a contains query. Matching is case-insensitive by default unless --case-sensitive is also passed."
    )]
    pub regex: bool,
    #[arg(
        long,
        default_value_t = 20,
        value_name = "N",
        help = "Maximum number of ranked matches to print"
    )]
    pub limit: usize,
    #[arg(
        long,
        help = "Use case-sensitive lookup matching",
        long_help = "Use case-sensitive lookup matching. By default lookup uses case-insensitive exact, prefix, contains, and regex matching."
    )]
    pub case_sensitive: bool,
    #[arg(
        long,
        default_value = "all",
        value_name = "SOURCE",
        help = "Knowledge source to search: all, memory, or terms"
    )]
    pub source: KnowledgeLookupSourceArg,
    #[arg(
        long,
        default_value = "both",
        value_name = "FIELD",
        help = "Text field to search: both, source, or target"
    )]
    pub field: KnowledgeLookupFieldArg,
    #[arg(
        long,
        help = "Emit structured JSON",
        long_help = "Emit structured JSON containing index_used, query, mode, total_matches, results, and diagnostics. Recommended for agent lookups."
    )]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeIndexRebuildCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "Source mod root directory",
        long_help = "Source mod root directory. The index is written to <MOD_ROOT>/.stringer/indexes/knowledge.sqlite."
    )]
    pub root: Utf8PathBuf,
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
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Override the global knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "Add a highest-priority knowledge root",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),

    #[error(transparent)]
    Adapt(#[from] AdaptError),
}

pub async fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Workspace { command } => {
            run_workspace(command).await?;
            Ok(())
        }
        Command::Adapt { command } => run_adapt(command).await,
        Command::Knowledge { command } => run_knowledge(command).await,
    }
}

async fn run_adapt(command: AdaptCommand) -> Result<(), CliError> {
    match command {
        AdaptCommand::Import(command) => {
            let input = command.input;
            let game = command
                .game
                .as_deref()
                .map(parse_game_release_name)
                .transpose()?
                .map(game_release_name)
                .map(str::to_string);
            let catalog = read_adapt_catalog(
                &input,
                AdaptImportOptions {
                    source_locale: command.source_locale,
                    target_locale: command.target_locale,
                    game,
                    format: command.format.into(),
                },
            )?;
            let (summary, action, output) = if let Some(output) = command.out {
                (write_memory_jsonl(&catalog, &output)?, "wrote", output)
            } else {
                let root = command
                    .global_knowledge_root
                    .or(load_global_knowledge_root(None)?)
                    .ok_or(WorkspaceError::MissingSetting {
                        name: "knowledge.global_root",
                    })?;
                let output = default_adapt_memory_path(&root, &input)?;
                (merge_memory_jsonl(&catalog, &output)?, "merged", output)
            };
            println!(
                "adapted {} entries, {} {} memory entries to {}, skipped {} entries, reported {} diagnostics",
                summary.total_entries,
                action,
                summary.written_entries,
                output,
                summary.skipped_entries,
                summary.diagnostics
            );
            Ok(())
        }
    }
}

async fn run_knowledge(command: KnowledgeCommand) -> Result<(), CliError> {
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
                "annotated {} entries, added {} hints, wrote {} diagnostics, auto-filled {} entries",
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
                mode: lookup_mode(command.regex),
                source: command.source.into(),
                field: command.field.into(),
                limit: command.limit,
                case_sensitive: command.case_sensitive,
            })?;
            if command.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "index_used": lookup.index_used,
                        "query": lookup.query,
                        "mode": lookup.mode.as_str(),
                        "total_matches": lookup.total_matches,
                        "results": lookup.results,
                        "diagnostics": lookup.diagnostics,
                    }))
                    .map_err(|source| WorkspaceError::Json {
                        path: Utf8PathBuf::from("<stdout>"),
                        source,
                    })?
                );
            } else {
                println!(
                    "found {} matches, showing {}, and reported {} diagnostics",
                    lookup.total_matches,
                    lookup.results.len(),
                    lookup.diagnostics.len()
                );
                for result in lookup.results {
                    let detail = result
                        .quality
                        .or(result.status)
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        result.kind,
                        result.layer,
                        result.match_field,
                        detail,
                        result.source,
                        result.target
                    );
                }
            }
            Ok(())
        }
        KnowledgeCommand::Index { command } => run_knowledge_index(command).await,
    }
}

async fn run_knowledge_index(command: KnowledgeIndexCommand) -> Result<(), CliError> {
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

fn lookup_mode(regex: bool) -> LookupKnowledgeMode {
    if regex {
        LookupKnowledgeMode::Regex
    } else {
        LookupKnowledgeMode::Contains
    }
}

impl From<KnowledgeLookupSourceArg> for LookupKnowledgeSource {
    fn from(value: KnowledgeLookupSourceArg) -> Self {
        match value {
            KnowledgeLookupSourceArg::All => Self::All,
            KnowledgeLookupSourceArg::Memory => Self::Memory,
            KnowledgeLookupSourceArg::Terms => Self::Terms,
        }
    }
}

impl From<KnowledgeLookupFieldArg> for LookupKnowledgeField {
    fn from(value: KnowledgeLookupFieldArg) -> Self {
        match value {
            KnowledgeLookupFieldArg::Both => Self::Both,
            KnowledgeLookupFieldArg::Source => Self::Source,
            KnowledgeLookupFieldArg::Target => Self::Target,
        }
    }
}

impl From<AdaptFormatArg> for AdaptFormat {
    fn from(value: AdaptFormatArg) -> Self {
        match value {
            AdaptFormatArg::Eet => Self::EetBinary,
            AdaptFormatArg::EetXml => Self::EetXml,
            AdaptFormatArg::EetJson => Self::EetJson,
            AdaptFormatArg::XtSst => Self::XtSst,
        }
    }
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

fn default_adapt_memory_path(
    root: &Utf8PathBuf,
    input: &Utf8PathBuf,
) -> Result<Utf8PathBuf, WorkspaceError> {
    let file_name = input
        .file_name()
        .ok_or_else(|| WorkspaceError::InvalidLogicalPath {
            path: input.to_string(),
            message: "adapt input path must include a file name".to_string(),
        })?;
    Ok(root
        .join("memory")
        .join("adapt")
        .join(format!("{file_name}.jsonl")))
}
