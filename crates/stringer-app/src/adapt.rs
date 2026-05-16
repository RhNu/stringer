use stringer_adapt::{
    AdaptImportOptions, merge_memory_jsonl, read_adapt_catalog, write_memory_jsonl,
};
use stringer_workspace_api::{
    WorkspaceError, game_release_name, global_knowledge_root_from_source, parse_game_release_name,
};
use stringer_workspace_core::GlobalConfigSource;

use crate::dto::{AdaptImportRequest, AdaptImportResponse, AdaptImportSummary};
use crate::error::AppError;
use crate::paths::{default_adapt_memory_path, path};

pub async fn adapt_import(request: AdaptImportRequest) -> Result<AdaptImportResponse, AppError> {
    adapt_import_with_global_config_source(request, &GlobalConfigSource::Production).await
}

pub(crate) async fn adapt_import_with_global_config_source(
    request: AdaptImportRequest,
    global_config_source: &GlobalConfigSource,
) -> Result<AdaptImportResponse, AppError> {
    let input = path(request.input);
    let game = request
        .game
        .as_deref()
        .map(parse_game_release_name)
        .transpose()?
        .map(game_release_name)
        .map(str::to_string);
    let catalog = read_adapt_catalog(
        &input,
        AdaptImportOptions {
            source_locale: request.source_locale,
            target_locale: request.target_locale,
            game,
            format: request.format.into(),
        },
    )?;
    let (summary, action, output) = if let Some(output) = request.out {
        let output = path(output);
        (write_memory_jsonl(&catalog, &output)?, "wrote", output)
    } else {
        let root = global_knowledge_root_from_source(global_config_source)?.ok_or(
            WorkspaceError::MissingSetting {
                name: "user_knowledge_root",
            },
        )?;
        let output = default_adapt_memory_path(&root, &input)?;
        (merge_memory_jsonl(&catalog, &output)?, "merged", output)
    };
    Ok(AdaptImportResponse {
        summary: AdaptImportSummary {
            total_entries: summary.total_entries,
            written_entries: summary.written_entries,
            skipped_entries: summary.skipped_entries,
            diagnostics: summary.diagnostics,
        },
        action: action.to_string(),
        output: output.to_string(),
    })
}
