use stringer_workspace::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeLookup, KnowledgeSummary,
    LookupKnowledgeMode, LookupKnowledgeOptions, ValidateTranslationsOptions, WorkspaceError,
    annotate_translations, build_knowledge_index, lookup_knowledge, validate_translations,
};

use crate::dto::{
    KnowledgeAnnotateRequest, KnowledgeIndexRebuildRequest, KnowledgeIndexRebuildResponse,
    KnowledgeKindInput, KnowledgeLookupRequest, KnowledgeLookupResponse,
    KnowledgeLookupResultResponse, KnowledgeOperationResponse, KnowledgeValidateRequest,
};
use crate::error::{AppError, serialize_value};
use crate::paths::{path, project_root_or_current};
use crate::settings::load_settings_for_project;

pub fn knowledge_annotate(
    request: KnowledgeAnnotateRequest,
) -> Result<KnowledgeOperationResponse, AppError> {
    let summary = annotate_translations(AnnotateTranslationsOptions {
        project_root: project_root_or_current(request.project_root)?,
        workspace: path(request.workspace),
        skip_memory_fill: request.skip_fill_memory,
    })?;
    Ok(knowledge_operation_response(summary))
}

pub fn knowledge_validate(
    request: KnowledgeValidateRequest,
) -> Result<KnowledgeOperationResponse, AppError> {
    let summary = validate_translations(ValidateTranslationsOptions {
        project_root: project_root_or_current(request.project_root)?,
        workspace: path(request.workspace),
    })?;
    Ok(knowledge_operation_response(summary))
}

pub fn knowledge_lookup(
    request: KnowledgeLookupRequest,
) -> Result<KnowledgeLookupResponse, AppError> {
    let project_root = project_root_or_current(request.project_root)?;
    let settings = load_settings_for_project(&project_root, request.settings)?;
    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        project_root,
        settings,
        text: request.text,
        kind: request.kind.into(),
        context: lookup_context(request.record_type, request.subrecord),
        mode: lookup_mode(request.regex),
        source: request.source.into(),
        field: request.field.into(),
        limit: request.limit,
        case_sensitive: request.case_sensitive,
    })?;
    knowledge_lookup_response(lookup)
}

pub fn knowledge_index_rebuild(
    request: KnowledgeIndexRebuildRequest,
) -> Result<KnowledgeIndexRebuildResponse, AppError> {
    let project_root = project_root_or_current(request.project_root)?;
    let settings = load_settings_for_project(&project_root, request.settings)?;
    let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
        project_root,
        settings,
    })?;
    Ok(KnowledgeIndexRebuildResponse {
        files: summary.files,
        terms: summary.terms,
        memory: summary.memory,
        rules: summary.rules,
        diagnostics: summary.diagnostics,
    })
}

pub fn parse_knowledge_kind(value: &str) -> Result<KnowledgeKindInput, AppError> {
    match value {
        "plugin" => Ok(KnowledgeKindInput::Plugin),
        "strings" => Ok(KnowledgeKindInput::Strings),
        "scaleform" => Ok(KnowledgeKindInput::Scaleform),
        "pex" => Ok(KnowledgeKindInput::Pex),
        _ => Err(WorkspaceError::InvalidSetting {
            name: "kind",
            value: value.to_string(),
        }
        .into()),
    }
}

fn knowledge_operation_response(summary: KnowledgeSummary) -> KnowledgeOperationResponse {
    KnowledgeOperationResponse {
        entries: summary.entries,
        annotations: summary.annotations,
        diagnostics: summary.diagnostics,
        auto_filled: summary.auto_filled,
        knowledge_diagnostics: summary.knowledge_diagnostics.len(),
        index_used: summary.index_used,
    }
}

fn knowledge_lookup_response(lookup: KnowledgeLookup) -> Result<KnowledgeLookupResponse, AppError> {
    Ok(KnowledgeLookupResponse {
        query: lookup.query,
        mode: lookup.mode.as_str().to_string(),
        total_matches: lookup.total_matches,
        results: lookup
            .results
            .into_iter()
            .map(|result| KnowledgeLookupResultResponse {
                kind: result.kind,
                id: result.id,
                layer: result.layer,
                source: result.source,
                target: result.target,
                match_field: result.match_field,
                match_kind: result.match_kind,
                score: result.score,
                quality: result.quality,
                status: result.status,
            })
            .collect(),
        diagnostics: lookup
            .diagnostics
            .into_iter()
            .map(|diagnostic| serialize_value("knowledge diagnostic", diagnostic))
            .collect::<Result<_, _>>()?,
        index_used: lookup.index_used,
    })
}

fn lookup_mode(regex: bool) -> LookupKnowledgeMode {
    if regex {
        LookupKnowledgeMode::Regex
    } else {
        LookupKnowledgeMode::Contains
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
