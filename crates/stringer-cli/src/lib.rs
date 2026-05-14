#![forbid(unsafe_code)]

mod app;
mod help;

pub use app::{
    AdaptCommand, AdaptFormatArg, AdaptImportCommand, Cli, CliError, Command, ExportCommand,
    ImportCommand, KnowledgeAnnotateCommand, KnowledgeCommand, KnowledgeIndexCommand,
    KnowledgeIndexRebuildCommand, KnowledgeLookupCommand, KnowledgeLookupFieldArg,
    KnowledgeLookupSourceArg, KnowledgeValidateCommand, run,
};
