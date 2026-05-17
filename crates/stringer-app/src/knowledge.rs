use stringer_knowledge::KnowledgeProgressEvent;
use stringer_knowledge::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, KnowledgeIndexBuildScope,
    KnowledgeLookup, KnowledgeSummary, KnowledgeTermDeleteOptions, KnowledgeTermEditSummary,
    KnowledgeTermInput as WorkspaceTermInput, KnowledgeTermStatus, KnowledgeTermsEditSummary,
    KnowledgeTermsUpsertOptions, LookupKnowledgeMode, LookupKnowledgeOptions,
    ValidateTranslationsOptions, annotate_translations_with_progress,
    build_knowledge_index_with_progress, delete_knowledge_term, lookup_knowledge,
    upsert_knowledge_terms, validate_translations_with_progress,
};
use stringer_workspace_api::WorkspaceError;
use stringer_workspace_core::{GlobalConfigSource, WorkspaceSettings, read_workspace_settings};

use crate::error::{AppError, serialize_value};
use crate::paths::{initialized_workspace_or_current, path, workspace_or_current};
use crate::settings::{knowledge_kind, lookup_field, lookup_source};
use stringer_interface::{
    KnowledgeAnnotateRequest, KnowledgeIndexRebuildRequest, KnowledgeIndexRebuildResponse,
    KnowledgeKindInput, KnowledgeLookupRequest, KnowledgeLookupResponse,
    KnowledgeLookupResultResponse, KnowledgeOperationResponse, KnowledgeTermDeleteRequest,
    KnowledgeTermEditResponse, KnowledgeTermStatusInput, KnowledgeTermUpsertRequest,
    KnowledgeTermsEditResponse, KnowledgeValidateRequest,
};

pub fn knowledge_annotate(
    request: KnowledgeAnnotateRequest,
) -> Result<KnowledgeOperationResponse, AppError> {
    knowledge_annotate_with_global_config_source(request, &GlobalConfigSource::Production)
}

pub fn knowledge_annotate_with_progress<F>(
    request: KnowledgeAnnotateRequest,
    progress: F,
) -> Result<KnowledgeOperationResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    knowledge_annotate_with_global_config_source_and_progress(
        request,
        &GlobalConfigSource::Production,
        progress,
    )
}

pub(crate) fn knowledge_annotate_with_global_config_source(
    request: KnowledgeAnnotateRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<KnowledgeOperationResponse, AppError> {
    knowledge_annotate_with_global_config_source_and_progress(request, global_config_source, |_| {})
}

pub(crate) fn knowledge_annotate_with_global_config_source_and_progress<F>(
    request: KnowledgeAnnotateRequest,
    global_config_source: &GlobalConfigSource,
    progress: F,
) -> Result<KnowledgeOperationResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    let summary = annotate_translations_with_progress(
        AnnotateTranslationsOptions {
            workspace: workspace_or_current(request.workspace)?,
            global_config_source: global_config_source.clone(),
            skip_memory_fill: request.skip_fill_memory,
        },
        progress,
    )?;
    Ok(knowledge_operation_response(summary))
}

pub fn knowledge_validate(
    request: KnowledgeValidateRequest,
) -> Result<KnowledgeOperationResponse, AppError> {
    knowledge_validate_with_global_config_source(request, &GlobalConfigSource::Production)
}

pub fn knowledge_validate_with_progress<F>(
    request: KnowledgeValidateRequest,
    progress: F,
) -> Result<KnowledgeOperationResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    knowledge_validate_with_global_config_source_and_progress(
        request,
        &GlobalConfigSource::Production,
        progress,
    )
}

pub(crate) fn knowledge_validate_with_global_config_source(
    request: KnowledgeValidateRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<KnowledgeOperationResponse, AppError> {
    knowledge_validate_with_global_config_source_and_progress(request, global_config_source, |_| {})
}

pub(crate) fn knowledge_validate_with_global_config_source_and_progress<F>(
    request: KnowledgeValidateRequest,
    global_config_source: &GlobalConfigSource,
    progress: F,
) -> Result<KnowledgeOperationResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    let summary = validate_translations_with_progress(
        ValidateTranslationsOptions {
            workspace: workspace_or_current(request.workspace)?,
            global_config_source: global_config_source.clone(),
        },
        progress,
    )?;
    Ok(knowledge_operation_response(summary))
}

pub fn knowledge_lookup(
    request: KnowledgeLookupRequest,
) -> Result<KnowledgeLookupResponse, AppError> {
    knowledge_lookup_with_global_config_source(request, &GlobalConfigSource::Production)
}

pub(crate) fn knowledge_lookup_with_global_config_source(
    request: KnowledgeLookupRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<KnowledgeLookupResponse, AppError> {
    let workspace = initialized_workspace_or_current(request.workspace)?;
    let settings = read_workspace_settings(&workspace)?;
    let lookup = lookup_knowledge(LookupKnowledgeOptions {
        workspace,
        settings,
        global_config_source: global_config_source.clone(),
        text: request.text,
        kind: knowledge_kind(request.kind),
        context: lookup_context(request.record_type, request.subrecord),
        mode: lookup_mode(request.regex),
        source: lookup_source(request.source),
        field: lookup_field(request.field),
        limit: request.limit,
        case_sensitive: request.case_sensitive,
    })?;
    knowledge_lookup_response(lookup)
}

pub fn knowledge_index_rebuild(
    request: KnowledgeIndexRebuildRequest,
) -> Result<KnowledgeIndexRebuildResponse, AppError> {
    knowledge_index_rebuild_with_global_config_source(request, &GlobalConfigSource::Production)
}

pub fn knowledge_index_rebuild_with_progress<F>(
    request: KnowledgeIndexRebuildRequest,
    progress: F,
) -> Result<KnowledgeIndexRebuildResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    knowledge_index_rebuild_with_global_config_source_and_progress(
        request,
        &GlobalConfigSource::Production,
        progress,
    )
}

pub(crate) fn knowledge_index_rebuild_with_global_config_source(
    request: KnowledgeIndexRebuildRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<KnowledgeIndexRebuildResponse, AppError> {
    knowledge_index_rebuild_with_global_config_source_and_progress(
        request,
        global_config_source,
        |_| {},
    )
}

pub(crate) fn knowledge_index_rebuild_with_global_config_source_and_progress<F>(
    request: KnowledgeIndexRebuildRequest,
    global_config_source: &GlobalConfigSource,
    progress: F,
) -> Result<KnowledgeIndexRebuildResponse, AppError>
where
    F: FnMut(KnowledgeProgressEvent),
{
    let workspace = initialized_workspace_or_current(request.workspace)?;
    let settings = read_workspace_settings(&workspace)?;
    let summary = build_knowledge_index_with_progress(
        BuildKnowledgeIndexOptions {
            workspace,
            settings,
            global_config_source: global_config_source.clone(),
            scope: KnowledgeIndexBuildScope::All,
        },
        progress,
    )?;
    Ok(KnowledgeIndexRebuildResponse {
        files: summary.files,
        terms: summary.terms,
        memory: summary.memory,
        rules: summary.rules,
        diagnostics: summary.diagnostics,
        indexed_items: summary.indexed_items,
        fts_rows: summary.fts_rows,
        rebuild_reason: summary.rebuild_reason,
    })
}

pub fn knowledge_term_upsert(
    request: KnowledgeTermUpsertRequest,
) -> Result<KnowledgeTermsEditResponse, AppError> {
    let workspace = knowledge_term_workspace(request.workspace, request.rebuild_index)?;
    let settings = knowledge_term_settings(&workspace, request.rebuild_index)?;
    let summary = upsert_knowledge_terms(KnowledgeTermsUpsertOptions {
        workspace,
        file: request.file.map(path),
        terms: request
            .terms
            .into_iter()
            .map(workspace_term_input)
            .collect(),
        rebuild_index: request.rebuild_index,
        settings,
    })?;
    Ok(knowledge_terms_edit_response(summary))
}

pub fn knowledge_term_delete(
    request: KnowledgeTermDeleteRequest,
) -> Result<KnowledgeTermEditResponse, AppError> {
    let workspace = knowledge_term_workspace(request.workspace, request.rebuild_index)?;
    let settings = knowledge_term_settings(&workspace, request.rebuild_index)?;
    let summary = delete_knowledge_term(KnowledgeTermDeleteOptions {
        workspace,
        file: request.file.map(path),
        id: request.id,
        rebuild_index: request.rebuild_index,
        settings,
    })?;
    Ok(knowledge_term_edit_response(summary))
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

fn knowledge_term_edit_response(summary: KnowledgeTermEditSummary) -> KnowledgeTermEditResponse {
    let index_rebuilt = summary.index_summary.is_some();
    KnowledgeTermEditResponse {
        action: summary.action,
        id: summary.id,
        path: summary.path.as_str().replace('\\', "/"),
        index_rebuilt,
        index_summary: summary
            .index_summary
            .map(|summary| KnowledgeIndexRebuildResponse {
                files: summary.files,
                terms: summary.terms,
                memory: summary.memory,
                rules: summary.rules,
                diagnostics: summary.diagnostics,
                indexed_items: summary.indexed_items,
                fts_rows: summary.fts_rows,
                rebuild_reason: summary.rebuild_reason,
            }),
    }
}

fn knowledge_terms_edit_response(summary: KnowledgeTermsEditSummary) -> KnowledgeTermsEditResponse {
    let index_rebuilt = summary.index_summary.is_some();
    KnowledgeTermsEditResponse {
        action: summary.action,
        ids: summary.ids,
        count: summary.count,
        path: summary.path.as_str().replace('\\', "/"),
        index_rebuilt,
        index_summary: summary
            .index_summary
            .map(|summary| KnowledgeIndexRebuildResponse {
                files: summary.files,
                terms: summary.terms,
                memory: summary.memory,
                rules: summary.rules,
                diagnostics: summary.diagnostics,
                indexed_items: summary.indexed_items,
                fts_rows: summary.fts_rows,
                rebuild_reason: summary.rebuild_reason,
            }),
    }
}

fn workspace_term_input(term: stringer_interface::KnowledgeTermInput) -> WorkspaceTermInput {
    WorkspaceTermInput {
        id: term.id,
        source: term.source,
        target: term.target,
        aliases: term.aliases,
        case_sensitive: term.case_sensitive,
        status: knowledge_term_status(term.status),
        scope: term.scope,
        tags: term.tags,
        note: term.note,
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

fn knowledge_term_status(value: KnowledgeTermStatusInput) -> KnowledgeTermStatus {
    match value {
        KnowledgeTermStatusInput::Preferred => KnowledgeTermStatus::Preferred,
        KnowledgeTermStatusInput::Allowed => KnowledgeTermStatus::Allowed,
        KnowledgeTermStatusInput::Forbidden => KnowledgeTermStatus::Forbidden,
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

fn knowledge_term_workspace(
    workspace: Option<String>,
    rebuild_index: bool,
) -> Result<camino::Utf8PathBuf, WorkspaceError> {
    if rebuild_index {
        initialized_workspace_or_current(workspace)
    } else {
        workspace_or_current(workspace)
    }
}

fn knowledge_term_settings(
    workspace: &camino::Utf8Path,
    rebuild_index: bool,
) -> Result<Option<WorkspaceSettings>, WorkspaceError> {
    Ok(rebuild_index
        .then(|| read_workspace_settings(workspace))
        .transpose()?)
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
