use std::sync::Mutex;
use stringer_core::Language;
use stringer_plugin::GameRelease;
use stringer_workspace_api::{
    GlobalConfigSource, LoadWorkspaceSettingsOptions, WorkspaceSettingsOverrides,
    load_global_knowledge_root, load_workspace_settings, with_global_knowledge_defaults,
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
        global_config_source: GlobalConfigSource::ConfigFile(utf8(&config)),
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
        global_config_source: GlobalConfigSource::ConfigFile(utf8(&user_config)),
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
        global_config_source: GlobalConfigSource::ConfigFile(utf8(&user_config)),
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
fn load_settings_uses_fixed_global_knowledge_root_without_reading_user_config() {
    let root = TempRoot::new("settings-fixed-global-root");
    let _env = PoisonedStringerConfig::new(&root.path().join("poison/config.toml"));
    let missing_config = root.path().join("missing/config.toml");
    let fake_global = root.path().join("fake-global");

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(Some(utf8(&fake_global))),
        workspace_config_path: Some(utf8(&missing_config)),
        overrides: WorkspaceSettingsOverrides {
            game_release: Some(GameRelease::SkyrimSe),
            asset_language: Some(Language::English),
            source_locale: Some("en".to_string()),
            target_locale: Some("zh-Hans".to_string()),
        },
    })
    .unwrap_err();

    assert!(settings.to_string().contains("failed to read"));

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        global_config_source: GlobalConfigSource::FixedKnowledgeRoot(Some(utf8(&fake_global))),
        workspace_config_path: None,
        overrides: WorkspaceSettingsOverrides {
            game_release: Some(GameRelease::SkyrimSe),
            asset_language: Some(Language::English),
            source_locale: Some("en".to_string()),
            target_locale: Some("zh-Hans".to_string()),
        },
    })
    .unwrap();

    assert_eq!(settings.global_knowledge_root, Some(utf8(&fake_global)));
}

#[test]
fn with_global_knowledge_defaults_can_disable_global_knowledge() {
    let root = TempRoot::new("settings-disabled-global-root");
    let _env = PoisonedStringerConfig::new(&root.path().join("poison/config.toml"));
    let mut settings = settings();
    settings.global_knowledge_root = None;

    let settings =
        with_global_knowledge_defaults(settings, &GlobalConfigSource::FixedKnowledgeRoot(None))
            .unwrap();

    assert_eq!(settings.global_knowledge_root, None);
}

static STRINGER_CONFIG_ENV: Mutex<()> = Mutex::new(());

struct PoisonedStringerConfig {
    _guard: std::sync::MutexGuard<'static, ()>,
    previous: Option<std::ffi::OsString>,
}

impl PoisonedStringerConfig {
    fn new(path: &std::path::Path) -> Self {
        let guard = STRINGER_CONFIG_ENV.lock().unwrap();
        write_text(path, "game_release = [not valid]");
        let previous = std::env::var_os("STRINGER_CONFIG");
        // SAFETY: Settings tests that mutate STRINGER_CONFIG hold this test-local
        // mutex guard for the duration of the mutation and restore it on drop.
        unsafe {
            std::env::set_var("STRINGER_CONFIG", path);
        }
        Self {
            _guard: guard,
            previous,
        }
    }
}

impl Drop for PoisonedStringerConfig {
    fn drop(&mut self) {
        // SAFETY: See PoisonedStringerConfig::new; the same mutex guard is still
        // held while restoring the process environment.
        unsafe {
            match &self.previous {
                Some(previous) => std::env::set_var("STRINGER_CONFIG", previous),
                None => std::env::remove_var("STRINGER_CONFIG"),
            }
        }
    }
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
