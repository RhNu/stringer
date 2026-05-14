//! Shared foundations for Stringer crates.

pub mod binary;

mod asset;
mod diagnostic;
mod error;
mod language;
mod string_entry;

pub use asset::{FileAsset, FileBundle, FileFormat, FileRole};
pub use diagnostic::{Diagnostic, DiagnosticSeverity, SourceSpan};
pub use error::StringerCoreError;
pub use language::Language;
pub use string_entry::{
    PexCallContext, PexConcatMetadata, PexConcatPart, PexFunctionKind, PexOperandPath,
    PexStringMetadata, PluginStringMetadata, PluginStringStorage, ScaleformStringMetadata,
    StringEntry, StringEntryBundle, StringEntryContext, StringEntrySource, StringEntryView,
};
