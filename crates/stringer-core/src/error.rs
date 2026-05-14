use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StringerCoreError {
    #[error("duplicate logical file path in bundle: {path}")]
    DuplicatePath { path: String },
}
