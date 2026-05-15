use stringer_workspace::{
    WorkspaceError, WorkspaceSettingsOverrides, parse_game_release_name, parse_language_name,
};

pub(crate) fn overrides(
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
) -> Result<WorkspaceSettingsOverrides, WorkspaceError> {
    Ok(WorkspaceSettingsOverrides {
        game_release: game_release
            .as_deref()
            .map(parse_game_release_name)
            .transpose()?,
        asset_language: asset_language
            .as_deref()
            .map(parse_language_name)
            .transpose()?,
        source_locale,
        target_locale,
    })
}
