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
        overrides: settings.overrides()?,
    })?)
}

impl SettingsInput {
    fn overrides(self) -> Result<WorkspaceSettingsOverrides, WorkspaceError> {
        Ok(WorkspaceSettingsOverrides {
            game_release: self
                .game_release
                .as_deref()
                .map(parse_game_release_name)
                .transpose()?,
            asset_language: self
                .asset_language
                .as_deref()
                .map(parse_language_name)
                .transpose()?,
            source_locale: self.source_locale,
            target_locale: self.target_locale,
        })
    }
}

impl From<AdaptFormatInput> for AdaptFormat {
    fn from(value: AdaptFormatInput) -> Self {
        match value {
            AdaptFormatInput::Eet => Self::EetBinary,
            AdaptFormatInput::EetXml => Self::EetXml,
            AdaptFormatInput::EetJson => Self::EetJson,
            AdaptFormatInput::XtSst => Self::XtSst,
        }
    }
}

impl From<KnowledgeKindInput> for PipelineEntryKind {
    fn from(value: KnowledgeKindInput) -> Self {
        match value {
            KnowledgeKindInput::Plugin => Self::Plugin,
            KnowledgeKindInput::Strings => Self::Strings,
            KnowledgeKindInput::Scaleform => Self::Scaleform,
            KnowledgeKindInput::Pex => Self::Pex,
        }
    }
}

impl From<KnowledgeLookupSourceInput> for LookupKnowledgeSource {
    fn from(value: KnowledgeLookupSourceInput) -> Self {
        match value {
            KnowledgeLookupSourceInput::All => Self::All,
            KnowledgeLookupSourceInput::Memory => Self::Memory,
            KnowledgeLookupSourceInput::Terms => Self::Terms,
        }
    }
}

impl From<KnowledgeLookupFieldInput> for LookupKnowledgeField {
    fn from(value: KnowledgeLookupFieldInput) -> Self {
        match value {
            KnowledgeLookupFieldInput::Both => Self::Both,
            KnowledgeLookupFieldInput::Source => Self::Source,
            KnowledgeLookupFieldInput::Target => Self::Target,
        }
    }
}
