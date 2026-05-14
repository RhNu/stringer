use std::io;

use camino::Utf8PathBuf;
use stringer_core::StringerCoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to walk `{path}`: {source}")]
    Walk {
        path: String,
        #[source]
        source: walkdir::Error,
    },

    #[error("unsupported archive format `{path}`")]
    UnsupportedArchive { path: Utf8PathBuf },

    #[error("failed to read archive `{path}`: {message}")]
    Archive { path: Utf8PathBuf, message: String },

    #[error(transparent)]
    Bundle(#[from] StringerCoreError),
}
