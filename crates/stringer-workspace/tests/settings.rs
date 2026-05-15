use stringer_core::Language;
use stringer_plugin::GameRelease;
use stringer_workspace::{
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
        project_config_path: None,
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
    assert_eq!(settings.global_knowledge_root, None);
}

#[test]
fn load_settings_applies_user_project_and_cli_precedence() {
    let root = TempRoot::new("settings-precedence");
    let user_config = root.path().join("user/config.toml");
    let project_config = root.path().join("project/stringer.toml");
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
        &project_config,
        r#"
game_release = "SkyrimSe"
asset_language = "Chinese"
target_locale = "zh-Hans"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        user_config_path: Some(utf8(&user_config)),
        project_config_path: Some(utf8(&project_config)),
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
fn load_settings_rejects_project_config_global_knowledge_root() {
    let root = TempRoot::new("settings-project-global-rejected");
    let user_config = root.path().join("user/config.toml");
    let project_config = root.path().join("project/stringer.toml");
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
        &project_config,
        r#"
[knowledge]
global_root = "project-knowledge"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        user_config_path: Some(utf8(&user_config)),
        project_config_path: Some(utf8(&project_config)),
        overrides: WorkspaceSettingsOverrides::default(),
    })
    .unwrap_err()
    .to_string();

    assert!(settings.contains("failed to parse TOML config"));
    assert!(settings.contains("unknown field `knowledge`"));
}

#[test]
fn load_global_knowledge_root_uses_configured_user_root_only() {
    let root = TempRoot::new("settings-global-root");
    let config = root.path().join("config/stringer.toml");
    write_text(
        &config,
        r#"
[knowledge]
global_root = "knowledge"
"#,
    );

    let global = load_global_knowledge_root(Some(utf8(&config))).unwrap();

    assert_eq!(global, Some(utf8(&root.path().join("config/knowledge"))));
}

#[test]
fn load_global_knowledge_root_returns_none_without_configured_root() {
    let root = TempRoot::new("settings-global-root-missing");
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

    assert_eq!(global, None);
}
