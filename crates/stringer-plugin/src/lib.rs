//! Bethesda plugin localization support for Stringer.

mod encoding;
mod error;
mod high_level;
mod plugin;
mod registry;
mod strings;
mod types;

pub use error::PluginError;
pub use high_level::{
    LocalizationBundle, LocalizationEntry, ReadOptions, WriteOptions, read_localization,
    write_localization,
};
pub use plugin::{
    ParsePluginOptions, ParsedPlugin, PluginLocalizationEntry, PluginRecord, parse_plugin_file,
    write_plugin_file,
};
pub use registry::{LocalizedField, skyrim_localized_fields};
pub use strings::{StringsFile, parse_strings_file, write_strings_file};
pub use types::{GameRelease, Language, LocalizedFieldSource, StringsKind};
