use std::fs;

use camino::Utf8PathBuf;
use serde::Deserialize;
use stringer_core::Language;
use stringer_plugin::GameRelease;

use crate::WorkspaceCoreError;

const CONFIG_ENV: &str = "STRINGER_CONFIG";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSettings {
    pub game_release: GameRelease,
    pub asset_language: Language,
    pub source_locale: String,
    pub target_locale: String,
    pub global_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceSettingsOverrides {
    pub game_release: Option<GameRelease>,
    pub asset_language: Option<Language>,
    pub source_locale: Option<String>,
    pub target_locale: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GlobalConfigSource {
    #[default]
    Production,
    ConfigFile(Utf8PathBuf),
    FixedKnowledgeRoot(Option<Utf8PathBuf>),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadWorkspaceSettingsOptions {
    pub global_config_source: GlobalConfigSource,
    pub workspace_config_path: Option<Utf8PathBuf>,
    pub overrides: WorkspaceSettingsOverrides,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct WorkspaceConfigFile {
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
}

struct LoadedConfigFile {
    config: ConfigFile,
    path: Option<Utf8PathBuf>,
}

struct LoadedGlobalConfig {
    config: ConfigFile,
    knowledge_root: Option<Utf8PathBuf>,
}

pub fn load_workspace_settings(
    options: LoadWorkspaceSettingsOptions,
) -> Result<WorkspaceSettings, WorkspaceCoreError> {
    let user = load_global_config(&options.global_config_source)?;
    let workspace_config = load_workspace_config_file(options.workspace_config_path)?;
    let user_config = user.config;
    let user_game_release = user_config
        .game_release
        .as_deref()
        .map(parse_game_release_name)
        .transpose()?;
    let workspace_game_release = workspace_config
        .game_release
        .as_deref()
        .map(parse_game_release_name)
        .transpose()?;
    let user_asset_language = user_config
        .asset_language
        .as_deref()
        .map(parse_language_name)
        .transpose()?;
    let workspace_asset_language = workspace_config
        .asset_language
        .as_deref()
        .map(parse_language_name)
        .transpose()?;

    Ok(WorkspaceSettings {
        game_release: options
            .overrides
            .game_release
            .or(workspace_game_release)
            .or(user_game_release)
            .ok_or(WorkspaceCoreError::MissingSetting {
                name: "game_release",
            })?,
        asset_language: options
            .overrides
            .asset_language
            .or(workspace_asset_language)
            .or(user_asset_language)
            .ok_or(WorkspaceCoreError::MissingSetting {
                name: "asset_language",
            })?,
        source_locale: take_setting(
            options.overrides.source_locale,
            workspace_config.source_locale,
            user_config.source_locale,
            "source_locale",
        )?,
        target_locale: take_setting(
            options.overrides.target_locale,
            workspace_config.target_locale,
            user_config.target_locale,
            "target_locale",
        )?,
        global_knowledge_root: user.knowledge_root,
    })
}

pub fn with_global_knowledge_defaults(
    mut settings: WorkspaceSettings,
    source: &GlobalConfigSource,
) -> Result<WorkspaceSettings, WorkspaceCoreError> {
    if settings.global_knowledge_root.is_none() {
        settings.global_knowledge_root = global_knowledge_root_from_source(source)?;
    }
    Ok(settings)
}

fn load_global_config(
    source: &GlobalConfigSource,
) -> Result<LoadedGlobalConfig, WorkspaceCoreError> {
    match source {
        GlobalConfigSource::Production => {
            let loaded = load_user_config_file(None)?;
            Ok(LoadedGlobalConfig {
                knowledge_root: user_knowledge_root(loaded.path.as_ref()),
                config: loaded.config,
            })
        }
        GlobalConfigSource::ConfigFile(path) => {
            let loaded = load_user_config_file(Some(path.clone()))?;
            Ok(LoadedGlobalConfig {
                knowledge_root: user_knowledge_root(loaded.path.as_ref()),
                config: loaded.config,
            })
        }
        GlobalConfigSource::FixedKnowledgeRoot(root) => Ok(LoadedGlobalConfig {
            config: ConfigFile::default(),
            knowledge_root: root.clone(),
        }),
    }
}

fn load_user_config_file(
    path: Option<Utf8PathBuf>,
) -> Result<LoadedConfigFile, WorkspaceCoreError> {
    let explicit = path.is_some();
    let Some(path) = path.or_else(default_config_path) else {
        return Ok(LoadedConfigFile {
            config: ConfigFile::default(),
            path: None,
        });
    };
    if !path.exists() && !explicit {
        return Ok(LoadedConfigFile {
            config: ConfigFile::default(),
            path: Some(path),
        });
    }
    let text = fs::read_to_string(&path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let config = toml::from_str(&text).map_err(|source| WorkspaceCoreError::ConfigToml {
        path: path.clone(),
        source,
    })?;
    Ok(LoadedConfigFile {
        config,
        path: Some(path),
    })
}

fn load_workspace_config_file(
    path: Option<Utf8PathBuf>,
) -> Result<WorkspaceConfigFile, WorkspaceCoreError> {
    let Some(path) = path else {
        return Ok(WorkspaceConfigFile::default());
    };
    let text = fs::read_to_string(&path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let config = toml::from_str(&text).map_err(|source| WorkspaceCoreError::ConfigToml {
        path: path.clone(),
        source,
    })?;
    Ok(config)
}

pub fn default_config_path() -> Option<Utf8PathBuf> {
    env_config_path().or_else(platform_default_config_path)
}

pub fn load_global_knowledge_root(
    config_path: Option<Utf8PathBuf>,
) -> Result<Option<Utf8PathBuf>, WorkspaceCoreError> {
    let source = match config_path {
        Some(path) => GlobalConfigSource::ConfigFile(path),
        None => GlobalConfigSource::Production,
    };
    global_knowledge_root_from_source(&source)
}

pub fn global_knowledge_root_from_source(
    source: &GlobalConfigSource,
) -> Result<Option<Utf8PathBuf>, WorkspaceCoreError> {
    match source {
        GlobalConfigSource::FixedKnowledgeRoot(root) => Ok(root.clone()),
        GlobalConfigSource::Production | GlobalConfigSource::ConfigFile(_) => {
            Ok(load_global_config(source)?.knowledge_root)
        }
    }
}

fn env_config_path() -> Option<Utf8PathBuf> {
    let raw = std::env::var_os(CONFIG_ENV)?;
    let path = std::path::PathBuf::from(raw);
    if path.as_os_str().is_empty() {
        return None;
    }
    Utf8PathBuf::from_path_buf(path).ok()
}

#[cfg(windows)]
fn platform_default_config_path() -> Option<Utf8PathBuf> {
    let documents = directories::UserDirs::new()?.document_dir()?.to_path_buf();
    Utf8PathBuf::from_path_buf(documents)
        .ok()
        .map(|path| path.join("My Games").join("Stringer").join("config.toml"))
}

#[cfg(not(windows))]
fn platform_default_config_path() -> Option<Utf8PathBuf> {
    let config = directories::BaseDirs::new()?.config_dir().to_path_buf();
    Utf8PathBuf::from_path_buf(config)
        .ok()
        .map(|path| path.join("stringer").join("config.toml"))
}

fn take_setting(
    override_value: Option<String>,
    workspace_value: Option<String>,
    user_value: Option<String>,
    name: &'static str,
) -> Result<String, WorkspaceCoreError> {
    let value = override_value
        .or(workspace_value)
        .or(user_value)
        .ok_or(WorkspaceCoreError::MissingSetting { name })?;
    if value.trim().is_empty() {
        return Err(WorkspaceCoreError::InvalidSetting { name, value });
    }
    Ok(value)
}

fn user_knowledge_root(config_path: Option<&Utf8PathBuf>) -> Option<Utf8PathBuf> {
    config_path
        .and_then(|path| path.parent().map(Utf8PathBuf::from))
        .map(|dir| dir.join("knowledge"))
}

pub fn parse_game_release_name(value: &str) -> Result<GameRelease, WorkspaceCoreError> {
    match normalize_name(value).as_str() {
        "skyrimle" => Ok(GameRelease::SkyrimLe),
        "skyrimse" => Ok(GameRelease::SkyrimSe),
        _ => Err(WorkspaceCoreError::InvalidSetting {
            name: "game_release",
            value: value.to_string(),
        }),
    }
}

pub fn parse_language_name(value: &str) -> Result<Language, WorkspaceCoreError> {
    let normalized = normalize_name(value);
    Language::ALL
        .into_iter()
        .find(|language| normalize_name(language.full_name()) == normalized)
        .ok_or_else(|| WorkspaceCoreError::InvalidSetting {
            name: "asset_language",
            value: value.to_string(),
        })
}

pub fn game_release_name(release: GameRelease) -> &'static str {
    match release {
        GameRelease::SkyrimLe => "SkyrimLe",
        GameRelease::SkyrimSe => "SkyrimSe",
    }
}

pub fn language_name(language: Language) -> &'static str {
    language.full_name()
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}
