#![forbid(unsafe_code)]

mod binary;
mod codec;
mod constants;
mod error;
mod filter;
mod high_level;
mod model;
mod opcode;

pub use error::PexError;
pub use high_level::{
    ParsedPex, PexStringBundle, ReadPexOptions, parse_pex_file, read_pex_strings, write_pex_file,
    write_pex_strings,
};
pub use model::{
    PexDebugFunctionInfo, PexDebugFunctionType, PexDebugInfo, PexFile, PexFunction, PexHeader,
    PexLocal, PexObject, PexParameter, PexProperty, PexState, PexStringId, PexUserFlag,
    PexVariable,
};
pub use opcode::{PexInstruction, PexOpcode, PexValue};
