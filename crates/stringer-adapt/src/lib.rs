#![forbid(unsafe_code)]

mod binary_ext;
mod eet;
mod hash;
mod memory;
mod model;
mod xml;
mod xt_sst;

pub use memory::{merge_memory_jsonl, write_memory_jsonl};
pub use model::{
    AdaptCatalog, AdaptDiagnostic, AdaptEntry, AdaptError, AdaptFormat, AdaptImportOptions,
    AdaptQuality, AdaptSummary,
};

pub(crate) use model::{ParsedEntry, catalog_from_entries, insert_non_empty, malformed};

use camino::Utf8Path;

pub fn read_adapt_catalog(
    path: impl AsRef<Utf8Path>,
    options: AdaptImportOptions,
) -> Result<AdaptCatalog, AdaptError> {
    let path = path.as_ref();
    match options.format {
        AdaptFormat::EetBinary => eet::read_binary(path, &options),
        AdaptFormat::EetXml => eet::read_xml(path, &options),
        AdaptFormat::EetJson => eet::read_json(path, &options),
        AdaptFormat::XtSst => xt_sst::read(path, &options),
    }
}
