use thiserror::Error;

use crate::{PexDebugFunctionType, PexOpcode, PexStringId};

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PexError {
    #[error("unsupported file `{path}`: {message}")]
    UnsupportedFile { path: String, message: String },

    #[error("invalid PEX magic 0x{value:08x}")]
    InvalidMagic { value: u32 },

    #[error("unsupported Skyrim PEX version {major}.{minor}")]
    UnsupportedVersion { major: u8, minor: u8 },

    #[error("unsupported PEX game id {game_id}")]
    UnsupportedGame { game_id: u16 },

    #[error(
        "truncated PEX while reading {what} at offset {offset}: needed {needed} bytes, remaining {remaining}"
    )]
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
        what: &'static str,
    },

    #[error("invalid UTF-8 in {what} at offset {offset}")]
    InvalidUtf8 { offset: usize, what: &'static str },

    #[error("{what} string id {} is outside string table length {table_len}", id.index())]
    StringIdOutOfRange {
        id: PexStringId,
        table_len: usize,
        what: &'static str,
    },

    #[error("unknown PEX opcode {opcode} at offset {offset}")]
    UnknownOpcode { offset: usize, opcode: u8 },

    #[error("unknown PEX value type {tag} at offset {offset}")]
    UnknownValueType { offset: usize, tag: u8 },

    #[error("invalid debug function type {tag} at offset {offset}")]
    InvalidDebugFunctionType { offset: usize, tag: u8 },

    #[error("malformed variadic count for opcode {opcode:?} at offset {offset}")]
    MalformedVariadicCount { offset: usize, opcode: PexOpcode },

    #[error(
        "object body size mismatch at offset {offset}: expected end {expected_end}, actual end {actual_end}"
    )]
    ObjectSizeMismatch {
        offset: usize,
        expected_end: usize,
        actual_end: usize,
    },

    #[error("trailing bytes after PEX body at offset {offset} of {len}")]
    TrailingBytes { offset: usize, len: usize },

    #[error("{what} count {len} exceeds u16::MAX")]
    CountTooLarge { what: &'static str, len: usize },

    #[error("{what} length {len} exceeds u16::MAX")]
    StringTooLong { what: &'static str, len: usize },

    #[error("string id {} is outside string table length {table_len}", id.index())]
    WriteStringIdOutOfRange { id: PexStringId, table_len: usize },

    #[error("object body length {len} exceeds u32::MAX")]
    ObjectTooLarge { len: usize },

    #[error("opcode {opcode:?} expects {expected} arguments but received {actual}")]
    InvalidInstructionArity {
        opcode: PexOpcode,
        expected: usize,
        actual: usize,
    },

    #[error("opcode {opcode:?} does not accept variadic arguments")]
    UnexpectedVariadicArguments { opcode: PexOpcode },

    #[error("variadic argument count {len} exceeds i32::MAX")]
    VariadicArgumentCountTooLarge { len: usize },

    #[error("auto property has no backing variable")]
    AutoPropertyMissingAutoVar,

    #[error("property is readable but has no getter function")]
    ReadablePropertyMissingGetter,

    #[error("property is writable but has no setter function")]
    WritablePropertyMissingSetter,

    #[error("property {} has an invalid PEX model: {reason}", property.index())]
    InvalidPropertyModel {
        property: PexStringId,
        reason: &'static str,
    },

    #[error("property {} {accessor} has an invalid signature: {reason}", property.index())]
    InvalidAccessorSignature {
        property: PexStringId,
        accessor: &'static str,
        reason: &'static str,
    },

    #[error(
        "debug function reference {function_type:?} object={} state={} function={} does not resolve",
        object_name.index(),
        state_name.index(),
        function_name.index()
    )]
    InvalidDebugFunctionReference {
        object_name: PexStringId,
        state_name: PexStringId,
        function_name: PexStringId,
        function_type: PexDebugFunctionType,
    },

    #[error(
        "debug line map for function {} has {actual} entries but function has {expected} instructions",
        function_name.index()
    )]
    DebugLineMapLengthMismatch {
        function_name: PexStringId,
        expected: usize,
        actual: usize,
    },

    #[error("string entry `{entry_id}` no longer resolves to a PEX instruction operand")]
    InvalidStringEntryBinding { entry_id: String },
}

impl PexError {
    pub(crate) fn unsupported_file(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::UnsupportedFile {
            path: path.into(),
            message: message.into(),
        }
    }
}
