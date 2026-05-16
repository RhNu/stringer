#![forbid(unsafe_code)]

mod candidates;
mod error;
mod index;
mod layers;
mod lookup;
mod session;
mod terms;
mod translations;

pub use error::KnowledgeError;
pub use lookup::{
    KnowledgeLookup, KnowledgeLookupResult, LookupKnowledgeField, LookupKnowledgeMode,
    LookupKnowledgeSource,
};
pub use terms::{
    KnowledgeTermDeleteOptions, KnowledgeTermEditSummary, KnowledgeTermInput, KnowledgeTermStatus,
    KnowledgeTermUpsertOptions, KnowledgeTermsEditSummary, KnowledgeTermsUpsertOptions,
    delete_knowledge_term, upsert_knowledge_term, upsert_knowledge_terms,
};
pub use translations::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeIndexBuildScope,
    KnowledgeIndexSummary, KnowledgeOperation, KnowledgeProgressEvent, KnowledgeProgressPhase,
    KnowledgeSummary, LoadKnowledgeLayersOptions, LoadedKnowledgeLayers, LookupKnowledgeOptions,
    ValidateTranslationsOptions, annotate_translations, annotate_translations_with_progress,
    build_knowledge_index, build_knowledge_index_with_progress, load_knowledge_layers,
    lookup_knowledge, validate_translations, validate_translations_with_progress,
};
