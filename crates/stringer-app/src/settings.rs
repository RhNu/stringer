use camino::Utf8PathBuf;
use stringer_adapt::AdaptFormat;
use stringer_knowledge::{LookupKnowledgeField, LookupKnowledgeSource};
use stringer_pipeline::PipelineEntryKind;
use stringer_workspace_api::WorkspaceError;
use stringer_workspace_core::{
    GlobalConfigSource, LoadWorkspaceSettingsOptions, WorkspaceSettings,
    WorkspaceSettingsOverrides, load_workspace_settings, parse_game_release_name,
    parse_language_name,
};

use crate::dto::{
    AdaptFormatInput, KnowledgeKindInput, KnowledgeLookupFieldInput, KnowledgeLookupSourceInput,
    SettingsInput,
};
use crate::paths::workspace_config_path;

pub(crate) fn load_settings_for_workspace(
    workspace: &Utf8PathBuf,
    settings: SettingsInput,
    global_config_source: &GlobalConfigSource,
) -> Result<WorkspaceSettings, WorkspaceError> {
    Ok(load_workspace_settings(LoadWorkspaceSettingsOptions {
        global_config_source: global_config_source.clone(),
        workspace_config_path: workspace_config_path(workspace),
        overrides: settings_overrides(settings)?,
    })?)
}

fn settings_overrides(
    settings: SettingsInput,
) -> Result<WorkspaceSettingsOverrides, WorkspaceError> {
    Ok(WorkspaceSettingsOverrides {
        game_release: settings
            .game_release
            .as_deref()
            .map(parse_game_release_name)
            .transpose()?,
        asset_language: settings
            .asset_language
            .as_deref()
            .map(parse_language_name)
            .transpose()?,
        source_locale: settings.source_locale,
        target_locale: settings.target_locale,
    })
}

pub(crate) fn adapt_format(value: AdaptFormatInput) -> AdaptFormat {
    match value {
        AdaptFormatInput::Eet => AdaptFormat::EetBinary,
        AdaptFormatInput::EetXml => AdaptFormat::EetXml,
        AdaptFormatInput::EetJson => AdaptFormat::EetJson,
        AdaptFormatInput::XtSst => AdaptFormat::XtSst,
    }
}

pub(crate) fn knowledge_kind(value: KnowledgeKindInput) -> PipelineEntryKind {
    match value {
        KnowledgeKindInput::Plugin => PipelineEntryKind::Plugin,
        KnowledgeKindInput::Strings => PipelineEntryKind::Strings,
        KnowledgeKindInput::Scaleform => PipelineEntryKind::Scaleform,
        KnowledgeKindInput::Pex => PipelineEntryKind::Pex,
    }
}

pub(crate) fn lookup_source(value: KnowledgeLookupSourceInput) -> LookupKnowledgeSource {
    match value {
        KnowledgeLookupSourceInput::All => LookupKnowledgeSource::All,
        KnowledgeLookupSourceInput::Memory => LookupKnowledgeSource::Memory,
        KnowledgeLookupSourceInput::Terms => LookupKnowledgeSource::Terms,
    }
}

pub(crate) fn lookup_field(value: KnowledgeLookupFieldInput) -> LookupKnowledgeField {
    match value {
        KnowledgeLookupFieldInput::Both => LookupKnowledgeField::Both,
        KnowledgeLookupFieldInput::Source => LookupKnowledgeField::Source,
        KnowledgeLookupFieldInput::Target => LookupKnowledgeField::Target,
    }
}
