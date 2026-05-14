use std::fs;

use camino::Utf8PathBuf;
use serde::Deserialize;
use stringer_core::Language;
use stringer_plugin::GameRelease;

use crate::WorkspaceError;

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
pub struct LoadWorkspaceSettingsOptions {
    pub config_path: Option<Utf8PathBuf>,
    pub overrides: WorkspaceSettingsOverrides,
}

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
    #[serde(default)]
    knowledge: KnowledgeConfigFile,
}

#[derive(Debug, Deserialize, Default)]
struct KnowledgeConfigFile {
    global_root: Option<String>,
}

struct LoadedConfigFile {
    config: ConfigFile,
    path: Option<Utf8PathBuf>,
}

pub fn load_workspace_settings(
    options: LoadWorkspaceSettingsOptions,
) -> Result<WorkspaceSettings, WorkspaceError> {
    let loaded = load_config_file(options.config_path)?;
    let config = loaded.config;
    let config_game_release = config
        .game_release
        .as_deref()
        .map(parse_game_release_name)
        .transpose()?;
    let config_asset_language = config
        .asset_language
        .as_deref()
        .map(parse_language_name)
        .transpose()?;

    Ok(WorkspaceSettings {
        game_release: options
            .overrides
            .game_release
            .or(config_game_release)
            .ok_or(WorkspaceError::MissingSetting {
                name: "game_release",
            })?,
        asset_language: options
            .overrides
            .asset_language
            .or(config_asset_language)
            .ok_or(WorkspaceError::MissingSetting {
                name: "asset_language",
            })?,
        source_locale: take_setting(
            options.overrides.source_locale,
            config.source_locale,
            "source_locale",
        )?,
        target_locale: take_setting(
            options.overrides.target_locale,
            config.target_locale,
            "target_locale",
        )?,
        global_knowledge_root: global_knowledge_root(loaded.path.as_ref(), config.knowledge),
    })
}

fn load_config_file(path: Option<Utf8PathBuf>) -> Result<LoadedConfigFile, WorkspaceError> {
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
    let text = fs::read_to_string(&path).map_err(|source| WorkspaceError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let config = toml::from_str(&text).map_err(|source| WorkspaceError::ConfigToml {
        path: path.clone(),
        source,
    })?;
    Ok(LoadedConfigFile {
        config,
        path: Some(path),
    })
}

pub fn default_config_path() -> Option<Utf8PathBuf> {
    platform_default_config_path()
}

pub fn load_global_knowledge_root(
    config_path: Option<Utf8PathBuf>,
) -> Result<Option<Utf8PathBuf>, WorkspaceError> {
    let explicit = config_path.is_some();
    let Some(path) = config_path.or_else(default_config_path) else {
        return Ok(None);
    };
    if path.exists() || explicit {
        let loaded = load_config_file(Some(path))?;
        return Ok(global_knowledge_root(
            loaded.path.as_ref(),
            loaded.config.knowledge,
        ));
    }
    Ok(path.parent().map(|parent| parent.join("knowledge")))
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
    config_value: Option<String>,
    name: &'static str,
) -> Result<String, WorkspaceError> {
    let value = override_value
        .or(config_value)
        .ok_or(WorkspaceError::MissingSetting { name })?;
    if value.trim().is_empty() {
        return Err(WorkspaceError::InvalidSetting { name, value });
    }
    Ok(value)
}

fn global_knowledge_root(
    config_path: Option<&Utf8PathBuf>,
    knowledge: KnowledgeConfigFile,
) -> Option<Utf8PathBuf> {
    let config_dir = config_path.and_then(|path| path.parent().map(Utf8PathBuf::from));
    if let Some(root) = knowledge.global_root {
        let root = Utf8PathBuf::from(root);
        if root.is_absolute() {
            return Some(root);
        }
        return config_dir.map(|dir| dir.join(root));
    }
    config_dir.map(|dir| dir.join("knowledge"))
}

pub fn parse_game_release_name(value: &str) -> Result<GameRelease, WorkspaceError> {
    match normalize_name(value).as_str() {
        "skyrimle" => Ok(GameRelease::SkyrimLe),
        "skyrimse" => Ok(GameRelease::SkyrimSe),
        _ => Err(WorkspaceError::InvalidSetting {
            name: "game_release",
            value: value.to_string(),
        }),
    }
}

pub fn parse_language_name(value: &str) -> Result<Language, WorkspaceError> {
    let normalized = normalize_name(value);
    Language::ALL
        .into_iter()
        .find(|language| normalize_name(language.full_name()) == normalized)
        .ok_or_else(|| WorkspaceError::InvalidSetting {
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
