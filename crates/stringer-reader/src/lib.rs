#![forbid(unsafe_code)]

mod archive;
mod error;
mod paths;
mod reader;
mod source;

pub use error::ReaderError;
pub use reader::{ReadModOptions, ReadModResult, read_mod_root};
pub use source::{FileSource, FileSourceKind, FileSourceState};
