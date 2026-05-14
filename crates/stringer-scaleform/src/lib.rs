#![forbid(unsafe_code)]

mod translation;

pub use translation::{
    ScaleformEntry, ScaleformError, ScaleformTranslationBundle, ScaleformTranslationFile,
    parse_scaleform_translation_file, read_scaleform_translations,
    write_scaleform_translation_file, write_scaleform_translations,
};
