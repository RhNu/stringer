#![forbid(unsafe_code)]

mod error;
mod index;
mod layers;
mod lookup;
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
    KnowledgeIndexSummary, KnowledgeSummary, LoadKnowledgeLayersOptions, LoadedKnowledgeLayers,
    LookupKnowledgeOptions, ValidateTranslationsOptions, annotate_translations,
    build_knowledge_index, load_knowledge_layers, lookup_knowledge, validate_translations,
};
