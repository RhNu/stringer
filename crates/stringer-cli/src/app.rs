use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};

use camino::Utf8PathBuf;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use stringer_app::{
    AppError, adapt_import, knowledge_annotate_with_progress,
    knowledge_index_rebuild_with_progress, knowledge_lookup, knowledge_term_delete,
    knowledge_term_upsert, knowledge_validate_with_progress, parse_knowledge_kind,
};
use stringer_interface::{
    AdaptFormatInput, AdaptImportRequest, KnowledgeAnnotateRequest, KnowledgeIndexRebuildRequest,
    KnowledgeLookupFieldInput, KnowledgeLookupRequest, KnowledgeLookupSourceInput,
    KnowledgeTermDeleteRequest, KnowledgeTermInput, KnowledgeTermStatusInput,
    KnowledgeTermUpsertRequest, KnowledgeValidateRequest,
};
use thiserror::Error;

use crate::feedback::{Feedback, ProgressHandle, ProgressModeArg, init_tracing};
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
    #[arg(
        short = 'v',
        long,
        global = true,
        action = ArgAction::Count,
        help = "Increase default diagnostic logging; repeat for trace logs; RUST_LOG overrides"
    )]
    pub verbose: u8,
    #[arg(long, global = true, help = "Suppress progress and diagnostic logging")]
    pub quiet: bool,
    #[arg(
        long,
        global = true,
        default_value = "auto",
        value_name = "MODE",
        help = "Progress display mode: auto, always, or never"
    )]
    pub progress: ProgressModeArg,
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
        long_help = "Input format: eet, eet-xml, eet-json, or xt-sst."
    )]
    pub format: AdaptFormatArg,
    #[arg(
        long,
        value_name = "INPUT",
        help = "External translation resource to read",
        long_help = "External translation resource to read."
    )]
    pub input: Utf8PathBuf,
    #[arg(
        long,
        value_name = "MEMORY_JSONL",
        help = "Output Stringer memory JSONL path",
        long_help = "Output memory JSONL path; omit to write under the standard user knowledge directory."
    )]
    pub out: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Source locale to write into memory rows",
        long_help = "Source locale for generated memory rows, for example en."
    )]
    pub source_locale: String,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "Target locale to write into memory rows",
        long_help = "Target locale for generated memory rows, for example zh-Hans."
    )]
    pub target_locale: String,
    #[arg(
        long,
        value_name = "GAME",
        help = "Optional game context, for example SkyrimSe",
        long_help = "Optional game context, for example SkyrimSe."
    )]
    pub game: Option<String>,
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
    #[command(
        about = "Create, update, or delete workspace terminology",
        arg_required_else_help = true
    )]
    Term {
        #[command(subcommand)]
        command: KnowledgeTermCommand,
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

#[derive(Debug, Subcommand)]
pub enum KnowledgeTermCommand {
    #[command(about = "Create or replace a terminology entry")]
    Upsert(KnowledgeTermUpsertCommand),
    #[command(about = "Delete a terminology entry")]
    Delete(KnowledgeTermDeleteCommand),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum KnowledgeTermStatusArg {
    #[default]
    Preferred,
    Allowed,
    Forbidden,
}

#[derive(Debug, Parser)]
pub struct KnowledgeAnnotateCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory to annotate; defaults to the current directory."
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        help = "Do not fill empty translations from high-confidence memory",
        long_help = "Write hints without filling empty translations from high-confidence memory."
    )]
    pub skip_fill_memory: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeValidateCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory to validate; defaults to the current directory."
    )]
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct KnowledgeLookupCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory for knowledge lookup; defaults to the current directory."
    )]
    pub workspace: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TEXT",
        help = "Text query to search in knowledge source and target fields",
        long_help = "Text query; pass --regex to treat it as a regex pattern."
    )]
    pub text: String,
    #[arg(
        long,
        default_value = "plugin",
        value_name = "KIND",
        help = "Entry kind: plugin, strings, scaleform, or pex",
        long_help = "Entry kind: plugin, strings, scaleform, or pex."
    )]
    pub kind: String,
    #[arg(
        long,
        value_name = "RECORD_TYPE",
        help = "Plugin record type, for example WEAP",
        long_help = "Plugin record type, for example WEAP, ARMO, or NPC_."
    )]
    pub record_type: Option<String>,
    #[arg(
        long,
        value_name = "SUBRECORD",
        help = "Plugin subrecord, for example FULL",
        long_help = "Plugin subrecord, for example FULL or DESC."
    )]
    pub subrecord: Option<String>,
    #[arg(
        long,
        help = "Treat --text as a regex pattern instead of a contains query",
        long_help = "Treat --text as a regex pattern."
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
        long_help = "Use case-sensitive lookup matching."
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
        long_help = "Emit structured JSON for agent lookups."
    )]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeIndexRebuildCommand {
    #[arg(
        long,
        default_value = ".",
        value_name = "WORKSPACE",
        help = "Translation workspace directory",
        long_help = "Translation workspace directory for the derived knowledge index; defaults to the current directory."
    )]
    pub workspace: Utf8PathBuf,
}

#[derive(Debug, Parser)]
pub struct KnowledgeTermUpsertCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "TERMS_TOML")]
    pub file: Option<Utf8PathBuf>,
    #[arg(long, value_name = "ID")]
    pub id: String,
    #[arg(long, value_name = "TEXT")]
    pub source: String,
    #[arg(long, value_name = "TEXT")]
    pub target: String,
    #[arg(long, default_value = "preferred", value_name = "STATUS")]
    pub status: KnowledgeTermStatusArg,
    #[arg(long = "alias", value_name = "TEXT")]
    pub aliases: Vec<String>,
    #[arg(long)]
    pub case_sensitive: bool,
    #[arg(long, value_name = "JSON")]
    pub scope_json: Option<String>,
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,
    #[arg(long, value_name = "TEXT")]
    pub note: Option<String>,
    #[arg(long)]
    pub rebuild_index: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeTermDeleteCommand {
    #[arg(long, default_value = ".", value_name = "WORKSPACE")]
    pub workspace: Utf8PathBuf,
    #[arg(long, value_name = "TERMS_TOML")]
    pub file: Option<Utf8PathBuf>,
    #[arg(long, value_name = "ID")]
    pub id: String,
    #[arg(long)]
    pub rebuild_index: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    App(Box<AppError>),

    #[error("failed to read `{path}`: {source}")]
    ReadInput {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to process JSON `{path}`: {source}")]
    Json {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("{message}")]
    InvalidArguments { message: String },
}

impl From<AppError> for CliError {
    fn from(source: AppError) -> Self {
        Self::App(Box::new(source))
    }
}

pub async fn run(cli: Cli) -> Result<(), CliError> {
    validate_feedback_args(&cli)?;
    init_tracing(cli.verbose, cli.quiet);
    let feedback = Feedback::new(cli.quiet, cli.progress);
    match cli.command {
        Command::Workspace { command } => {
            run_workspace(command, &feedback).await?;
            Ok(())
        }
        Command::Adapt { command } => run_adapt(command, &feedback).await,
        Command::Knowledge { command } => run_knowledge(command, &feedback).await,
    }
}

async fn run_adapt(command: AdaptCommand, feedback: &Feedback) -> Result<(), CliError> {
    match command {
        AdaptCommand::Import(command) => {
            let status = feedback.command("adapt import");
            let imported = adapt_import(AdaptImportRequest {
                format: command.format.into(),
                input: command.input.to_string(),
                out: command.out.map(|path| path.to_string()),
                source_locale: command.source_locale,
                target_locale: command.target_locale,
                game: command.game,
            })
            .await?;
            status.finish();
            println!(
                "adapted {} entries, {} {} memory entries to {}, skipped {} entries, reported {} diagnostics",
                imported.summary.total_entries,
                imported.action,
                imported.summary.written_entries,
                imported.output,
                imported.summary.skipped_entries,
                imported.summary.diagnostics
            );
            Ok(())
        }
    }
}

async fn run_knowledge(command: KnowledgeCommand, feedback: &Feedback) -> Result<(), CliError> {
    match command {
        KnowledgeCommand::Annotate(command) => {
            let mut progress = None;
            let summary = knowledge_annotate_with_progress(
                KnowledgeAnnotateRequest {
                    workspace: Some(command.workspace.to_string()),
                    skip_fill_memory: command.skip_fill_memory,
                },
                |event| {
                    update_knowledge_progress(feedback, "knowledge annotate", &mut progress, event)
                },
            )?;
            finish_progress(progress);
            println!(
                "annotated {} entries, added {} hints, wrote {} diagnostics, auto-filled {} entries",
                summary.entries, summary.annotations, summary.diagnostics, summary.auto_filled
            );
            Ok(())
        }
        KnowledgeCommand::Validate(command) => {
            let mut progress = None;
            let summary = knowledge_validate_with_progress(
                KnowledgeValidateRequest {
                    workspace: Some(command.workspace.to_string()),
                },
                |event| {
                    update_knowledge_progress(feedback, "knowledge validate", &mut progress, event)
                },
            )?;
            finish_progress(progress);
            println!(
                "validated {} entries and wrote {} diagnostics",
                summary.entries, summary.diagnostics
            );
            Ok(())
        }
        KnowledgeCommand::Lookup(command) => {
            let status = feedback.command("knowledge lookup");
            let lookup = knowledge_lookup(KnowledgeLookupRequest {
                workspace: Some(command.workspace.to_string()),
                text: command.text,
                kind: parse_knowledge_kind(&command.kind)?,
                record_type: command.record_type,
                subrecord: command.subrecord,
                regex: command.regex,
                limit: command.limit,
                case_sensitive: command.case_sensitive,
                source: command.source.into(),
                field: command.field.into(),
            })?;
            status.finish();
            if command.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "index_used": lookup.index_used,
                        "query": lookup.query,
                        "mode": lookup.mode,
                        "total_matches": lookup.total_matches,
                        "results": lookup.results,
                        "diagnostics": lookup.diagnostics,
                    }))
                    .map_err(|source| CliError::Json {
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
        KnowledgeCommand::Index { command } => run_knowledge_index(command, feedback).await,
        KnowledgeCommand::Term { command } => run_knowledge_term(command, feedback).await,
    }
}

async fn run_knowledge_index(
    command: KnowledgeIndexCommand,
    feedback: &Feedback,
) -> Result<(), CliError> {
    match command {
        KnowledgeIndexCommand::Rebuild(command) => {
            let mut progress = None;
            let summary = knowledge_index_rebuild_with_progress(
                KnowledgeIndexRebuildRequest {
                    workspace: Some(command.workspace.to_string()),
                },
                |event| {
                    update_knowledge_progress(
                        feedback,
                        "knowledge index rebuild",
                        &mut progress,
                        event,
                    )
                },
            )?;
            finish_progress(progress);
            println!(
                "indexed {} files, {} terms, {} memory entries, {} rules, {} diagnostics",
                summary.files, summary.terms, summary.memory, summary.rules, summary.diagnostics
            );
            Ok(())
        }
    }
}

async fn run_knowledge_term(
    command: KnowledgeTermCommand,
    feedback: &Feedback,
) -> Result<(), CliError> {
    match command {
        KnowledgeTermCommand::Upsert(command) => {
            let status = feedback.command("knowledge term upsert");
            let response = knowledge_term_upsert(KnowledgeTermUpsertRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file.map(|path| path.to_string()),
                terms: vec![KnowledgeTermInput {
                    id: command.id,
                    source: command.source,
                    target: command.target,
                    aliases: command.aliases,
                    case_sensitive: command.case_sensitive,
                    status: command.status.into(),
                    scope: parse_scope_json(command.scope_json)?,
                    tags: command.tags,
                    note: command.note,
                }],
                rebuild_index: command.rebuild_index,
            })?;
            status.finish();
            if command.json {
                print_json(&response)?;
            } else {
                println!("upserted {} term(s) in {}", response.count, response.path);
            }
            Ok(())
        }
        KnowledgeTermCommand::Delete(command) => {
            let status = feedback.command("knowledge term delete");
            let response = knowledge_term_delete(KnowledgeTermDeleteRequest {
                workspace: Some(command.workspace.to_string()),
                file: command.file.map(|path| path.to_string()),
                id: command.id,
                rebuild_index: command.rebuild_index,
            })?;
            status.finish();
            if command.json {
                print_json(&response)?;
            } else {
                println!("deleted term {} from {}", response.id, response.path);
            }
            Ok(())
        }
    }
}

impl From<KnowledgeLookupSourceArg> for KnowledgeLookupSourceInput {
    fn from(value: KnowledgeLookupSourceArg) -> Self {
        match value {
            KnowledgeLookupSourceArg::All => Self::All,
            KnowledgeLookupSourceArg::Memory => Self::Memory,
            KnowledgeLookupSourceArg::Terms => Self::Terms,
        }
    }
}

impl From<KnowledgeLookupFieldArg> for KnowledgeLookupFieldInput {
    fn from(value: KnowledgeLookupFieldArg) -> Self {
        match value {
            KnowledgeLookupFieldArg::Both => Self::Both,
            KnowledgeLookupFieldArg::Source => Self::Source,
            KnowledgeLookupFieldArg::Target => Self::Target,
        }
    }
}

impl From<KnowledgeTermStatusArg> for KnowledgeTermStatusInput {
    fn from(value: KnowledgeTermStatusArg) -> Self {
        match value {
            KnowledgeTermStatusArg::Preferred => Self::Preferred,
            KnowledgeTermStatusArg::Allowed => Self::Allowed,
            KnowledgeTermStatusArg::Forbidden => Self::Forbidden,
        }
    }
}

impl From<AdaptFormatArg> for AdaptFormatInput {
    fn from(value: AdaptFormatArg) -> Self {
        match value {
            AdaptFormatArg::Eet => Self::Eet,
            AdaptFormatArg::EetXml => Self::EetXml,
            AdaptFormatArg::EetJson => Self::EetJson,
            AdaptFormatArg::XtSst => Self::XtSst,
        }
    }
}

fn parse_scope_json(text: Option<String>) -> Result<BTreeMap<String, Vec<String>>, CliError> {
    match text {
        Some(text) => serde_json::from_str(&text).map_err(|source| CliError::Json {
            path: Utf8PathBuf::from("<scope-json>"),
            source,
        }),
        None => Ok(BTreeMap::new()),
    }
}

pub(crate) fn read_input(path: &Utf8PathBuf) -> Result<String, CliError> {
    if path.as_str() == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|source| CliError::ReadInput {
                path: path.clone(),
                source,
            })?;
        return Ok(text);
    }
    fs::read_to_string(path).map_err(|source| CliError::ReadInput {
        path: path.clone(),
        source,
    })
}

pub(crate) fn print_json(value: &impl serde::Serialize) -> Result<(), CliError> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|source| CliError::Json {
            path: Utf8PathBuf::from("<stdout>"),
            source,
        })?
    );
    Ok(())
}

fn validate_feedback_args(cli: &Cli) -> Result<(), CliError> {
    if cli.quiet && cli.progress == ProgressModeArg::Always {
        return Err(CliError::InvalidArguments {
            message: "--quiet cannot be used with --progress always".to_string(),
        });
    }
    Ok(())
}

fn update_knowledge_progress(
    feedback: &Feedback,
    label: &str,
    handle: &mut Option<ProgressHandle>,
    event: stringer_app::KnowledgeProgressEvent,
) {
    match event.phase {
        stringer_app::KnowledgeProgressPhase::Started => {
            *handle = Some(feedback.progress(label, event.total.unwrap_or_default() as u64));
        }
        stringer_app::KnowledgeProgressPhase::Advanced => {
            if let Some(progress) = handle {
                progress.set_position(event.processed as u64);
                if let Some(message) = event.message {
                    progress.set_message(format!("{label}: {message}"));
                }
            }
        }
        stringer_app::KnowledgeProgressPhase::Finished => {
            if let Some(progress) = handle.take() {
                progress.finish();
            }
        }
    }
}

fn finish_progress(progress: Option<ProgressHandle>) {
    if let Some(progress) = progress {
        progress.finish();
    }
}
