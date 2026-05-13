use std::collections::HashMap;

use crate::{PexError, PexInstruction, PexValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PexStringId(u16);

impl PexStringId {
    pub const fn new(index: u16) -> Self {
        Self(index)
    }

    pub const fn index(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PexVersion {
    major: u8,
    minor: u8,
}

impl PexVersion {
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    pub const fn major(self) -> u8 {
        self.major
    }

    pub const fn minor(self) -> u8 {
        self.minor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexHeader {
    pex_version: PexVersion,
    compilation_time: u64,
    source_file_name: String,
    user_name: String,
    computer_name: String,
}

impl PexHeader {
    pub fn new_skyrim(
        compilation_time: u64,
        source_file_name: impl Into<String>,
        user_name: impl Into<String>,
        computer_name: impl Into<String>,
    ) -> Self {
        Self {
            pex_version: PexVersion::new(3, 2),
            compilation_time,
            source_file_name: source_file_name.into(),
            user_name: user_name.into(),
            computer_name: computer_name.into(),
        }
    }

    pub(crate) fn read(
        pex_version: PexVersion,
        compilation_time: u64,
        source_file_name: String,
        user_name: String,
        computer_name: String,
    ) -> Self {
        Self {
            pex_version,
            compilation_time,
            source_file_name,
            user_name,
            computer_name,
        }
    }

    pub const fn pex_version(&self) -> PexVersion {
        self.pex_version
    }

    pub const fn compilation_time(&self) -> u64 {
        self.compilation_time
    }

    pub fn source_file_name(&self) -> &str {
        &self.source_file_name
    }

    pub fn user_name(&self) -> &str {
        &self.user_name
    }

    pub fn computer_name(&self) -> &str {
        &self.computer_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PexUserFlag {
    pub name: PexStringId,
    pub bit_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexDebugInfo {
    pub modification_time: u64,
    pub functions: Vec<PexDebugFunctionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexDebugFunctionInfo {
    pub object_name: PexStringId,
    pub state_name: PexStringId,
    pub function_name: PexStringId,
    pub function_type: PexDebugFunctionType,
    pub instruction_line_map: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PexDebugFunctionType {
    Normal = 0,
    Getter = 1,
    Setter = 2,
}

#[derive(Debug, Clone)]
pub struct PexFile {
    header: PexHeader,
    strings: Vec<String>,
    string_lookup: HashMap<String, PexStringId>,
    pub debug_info: Option<PexDebugInfo>,
    pub user_flags: Vec<PexUserFlag>,
    pub objects: Vec<PexObject>,
}

impl PartialEq for PexFile {
    fn eq(&self, other: &Self) -> bool {
        self.header == other.header
            && self.strings == other.strings
            && self.debug_info == other.debug_info
            && self.user_flags == other.user_flags
            && self.objects == other.objects
    }
}

impl PexFile {
    pub fn new(header: PexHeader) -> Self {
        Self {
            header,
            strings: Vec::new(),
            string_lookup: HashMap::new(),
            debug_info: None,
            user_flags: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub const fn header(&self) -> &PexHeader {
        &self.header
    }

    pub fn intern(&mut self, text: impl AsRef<str>) -> Result<PexStringId, PexError> {
        let text = text.as_ref();
        if let Some(id) = self.string_lookup.get(text) {
            return Ok(*id);
        }
        ensure_u16("string table", self.strings.len() + 1)?;
        validate_counted_str("string table entry", text)?;
        let id = PexStringId::new(self.strings.len() as u16);
        self.strings.push(text.to_string());
        self.string_lookup.insert(text.to_string(), id);
        Ok(id)
    }

    pub fn string(&self, id: PexStringId) -> Option<&str> {
        self.strings.get(id.index() as usize).map(String::as_str)
    }

    pub fn string_table(&self) -> &[String] {
        &self.strings
    }

    pub(crate) fn from_parts(
        header: PexHeader,
        strings: Vec<String>,
        debug_info: Option<PexDebugInfo>,
        user_flags: Vec<PexUserFlag>,
        objects: Vec<PexObject>,
    ) -> Self {
        let string_lookup = strings
            .iter()
            .enumerate()
            .map(|(index, text)| (text.clone(), PexStringId::new(index as u16)))
            .collect();
        Self {
            header,
            strings,
            string_lookup,
            debug_info,
            user_flags,
            objects,
        }
    }

    pub(crate) fn replace_string(
        &mut self,
        id: PexStringId,
        text: impl Into<String>,
    ) -> Result<(), PexError> {
        let index = id.index() as usize;
        if index >= self.strings.len() {
            return Err(PexError::WriteStringIdOutOfRange {
                id,
                table_len: self.strings.len(),
            });
        }
        let text = text.into();
        validate_counted_str("string table entry", &text)?;
        let old = std::mem::replace(&mut self.strings[index], text.clone());
        if self.string_lookup.get(&old) == Some(&id) {
            self.string_lookup.remove(&old);
        }
        self.string_lookup.entry(text).or_insert(id);
        Ok(())
    }
}

impl Default for PexFile {
    fn default() -> Self {
        Self::new(PexHeader::new_skyrim(0, "", "", ""))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PexObject {
    pub name: PexStringId,
    pub parent_class_name: PexStringId,
    pub documentation_string: PexStringId,
    pub user_flags: u32,
    pub auto_state_name: PexStringId,
    pub variables: Vec<PexVariable>,
    pub properties: Vec<PexProperty>,
    pub states: Vec<PexState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PexVariable {
    pub name: PexStringId,
    pub type_name: PexStringId,
    pub user_flags: u32,
    pub default_value: PexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PexProperty {
    pub name: PexStringId,
    pub type_name: PexStringId,
    pub documentation_string: PexStringId,
    pub user_flags: u32,
    pub is_readable: bool,
    pub is_writable: bool,
    pub is_auto: bool,
    pub auto_var: Option<PexStringId>,
    pub read_function: Option<PexFunction>,
    pub write_function: Option<PexFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PexState {
    pub name: PexStringId,
    pub functions: Vec<PexFunction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PexFunction {
    pub name: PexStringId,
    pub return_type_name: PexStringId,
    pub documentation_string: PexStringId,
    pub user_flags: u32,
    pub is_global: bool,
    pub is_native: bool,
    pub parameters: Vec<PexParameter>,
    pub locals: Vec<PexLocal>,
    pub instructions: Vec<PexInstruction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PexParameter {
    pub name: PexStringId,
    pub type_name: PexStringId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PexLocal {
    pub name: PexStringId,
    pub type_name: PexStringId,
}

pub(crate) fn validate_counted_str(what: &'static str, text: &str) -> Result<(), PexError> {
    if text.len() > u16::MAX as usize {
        return Err(PexError::StringTooLong {
            what,
            len: text.len(),
        });
    }
    Ok(())
}

pub(crate) fn ensure_u16(what: &'static str, len: usize) -> Result<(), PexError> {
    if len > u16::MAX as usize {
        return Err(PexError::CountTooLarge { what, len });
    }
    Ok(())
}
