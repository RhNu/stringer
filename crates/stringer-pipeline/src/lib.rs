#![forbid(unsafe_code)]

mod error;
mod knowledge;
mod model;
mod processors;

pub use error::PipelineError;
pub use knowledge::{KnowledgeBase, KnowledgeLayer, ReplacementRule, Term, TranslationMemoryEntry};
pub use model::{
    PipelineAnnotation, PipelineDiagnostic, PipelineDiagnosticSeverity, PipelineEntry,
    PipelineEntryKind, PipelineOptions, PipelineReport, PipelineStage,
};
pub use processors::{
    BasicValidationProcessor, Pipeline, PipelineProcessor, ReplacementRuleProcessor,
    TerminologyProcessor, TranslationMemoryProcessor,
};
