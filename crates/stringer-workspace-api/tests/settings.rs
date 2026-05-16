use stringer_core::Language;
use stringer_plugin::GameRelease;
use stringer_workspace_api::{
    LoadWorkspaceSettingsOptions, WorkspaceSettingsOverrides, load_global_knowledge_root,
    load_workspace_settings,
};

#[allow(dead_code)]
mod support;

use support::*;

#[test]
fn load_settings_reads_toml_config_and_applies_explicit_overrides() {
    let root = TempRoot::new("settings");
    let config = root.path().join("config.toml");
    write_text(
        &config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        user_config_path: Some(utf8(&config)),
        workspace_config_path: None,
        overrides: WorkspaceSettingsOverrides {
            target_locale: Some("ja".to_string()),
            ..WorkspaceSettingsOverrides::default()
        },
    })
    .unwrap();

    assert_eq!(settings.game_release, GameRelease::SkyrimSe);
    assert_eq!(settings.asset_language, Language::English);
    assert_eq!(settings.source_locale, "en");
    assert_eq!(settings.target_locale, "ja");
    assert_eq!(
        settings.global_knowledge_root,
        Some(utf8(&root.path().join("knowledge")))
    );
}

#[test]
fn load_settings_applies_user_workspace_and_cli_precedence() {
    let root = TempRoot::new("settings-precedence");
    let user_config = root.path().join("user/config.toml");
    let workspace_config = root.path().join("workspace/stringer.toml");
    write_text(
        &user_config,
        r#"
game_release = "SkyrimLe"
asset_language = "English"
source_locale = "en"
target_locale = "de"
"#,
    );
    write_text(
        &workspace_config,
        r#"
game_release = "SkyrimSe"
asset_language = "Chinese"
target_locale = "zh-Hans"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        user_config_path: Some(utf8(&user_config)),
        workspace_config_path: Some(utf8(&workspace_config)),
        overrides: WorkspaceSettingsOverrides {
            asset_language: Some(Language::English),
            ..WorkspaceSettingsOverrides::default()
        },
    })
    .unwrap();

    assert_eq!(settings.game_release, GameRelease::SkyrimSe);
    assert_eq!(settings.asset_language, Language::English);
    assert_eq!(settings.source_locale, "en");
    assert_eq!(settings.target_locale, "zh-Hans");
}

#[test]
fn load_settings_ignores_legacy_workspace_knowledge_config() {
    let root = TempRoot::new("settings-workspace-knowledge-ignored");
    let user_config = root.path().join("user/config.toml");
    let workspace_config = root.path().join("workspace/stringer.toml");
    write_text(
        &user_config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"

[knowledge]
global_root = "user-knowledge"
"#,
    );
    write_text(
        &workspace_config,
        r#"
target_locale = "ja"

[knowledge]
global_root = "workspace-knowledge"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        user_config_path: Some(utf8(&user_config)),
        workspace_config_path: Some(utf8(&workspace_config)),
        overrides: WorkspaceSettingsOverrides::default(),
    })
    .unwrap();

    assert_eq!(settings.target_locale, "ja");
    assert_eq!(
        settings.global_knowledge_root,
        Some(utf8(&root.path().join("user/knowledge")))
    );
}

#[test]
fn load_global_knowledge_root_uses_standard_user_directory_without_configured_root() {
    let root = TempRoot::new("settings-global-root-default");
    let config = root.path().join("config/stringer.toml");
    write_text(
        &config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );

    let global = load_global_knowledge_root(Some(utf8(&config))).unwrap();

    assert_eq!(global, Some(utf8(&root.path().join("config/knowledge"))));
}

#[test]
fn load_global_knowledge_root_ignores_legacy_configured_root() {
    let root = TempRoot::new("settings-global-root-legacy");
    let config = root.path().join("config/stringer.toml");
    write_text(
        &config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"

[knowledge]
global_root = "custom-knowledge"
"#,
    );

    let global = load_global_knowledge_root(Some(utf8(&config))).unwrap();

    assert_eq!(global, Some(utf8(&root.path().join("config/knowledge"))));
}
