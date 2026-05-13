use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PluginError {
    #[error("malformed strings file `{path}`: {message}")]
    MalformedStrings { path: String, message: String },

    #[error("duplicate string id {id} in `{path}`")]
    DuplicateStringId { path: String, id: u32 },

    #[error("malformed plugin file `{path}`: {message}")]
    MalformedPlugin { path: String, message: String },

    #[error("duplicate strings file for {mod_name} {language} {kind}: {path}")]
    DuplicateStringsFile {
        mod_name: String,
        language: String,
        kind: String,
        path: String,
    },

    #[error("missing string id {id} in {kind} for {language}: {path}")]
    MissingStringId {
        path: String,
        language: String,
        kind: String,
        id: u32,
    },

    #[error(
        "invalid localized entry state in `{path}` for {record_type}.{subrecord} {form_id:#010X}: {message}"
    )]
    InvalidLocalizedEntryState {
        path: String,
        record_type: String,
        form_id: u32,
        subrecord: String,
        message: String,
    },

    #[error("ambiguous plugin files in bundle: {paths:?}")]
    AmbiguousPluginFiles { paths: Vec<String> },

    #[error("unsupported file `{path}`: {message}")]
    UnsupportedFile { path: String, message: String },

    #[error("encoding error for {language} in {release}: {message}")]
    Encoding {
        release: String,
        language: String,
        message: String,
    },
}

impl PluginError {
    pub(crate) fn malformed_strings(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MalformedStrings {
            path: path.into(),
            message: message.into(),
        }
    }

    pub(crate) fn malformed_plugin(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MalformedPlugin {
            path: path.into(),
            message: message.into(),
        }
    }
}
