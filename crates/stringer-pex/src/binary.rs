use stringer_core::binary::{
    BinaryError, BinaryReader as CoreBinaryReader, BinaryWriter as CoreBinaryWriter, Endian,
};

use crate::constants::PEX_MAGIC;
use crate::{PexError, PexStringId};

pub(crate) struct BinaryWriter {
    inner: CoreBinaryWriter,
}

impl BinaryWriter {
    pub(crate) fn big_endian() -> Self {
        Self {
            inner: CoreBinaryWriter::new(Endian::Big),
        }
    }

    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.inner.into_bytes()
    }

    pub(crate) fn extend(&mut self, bytes: impl AsRef<[u8]>) {
        self.inner.extend(bytes);
    }

    pub(crate) fn write_u8(&mut self, value: u8) {
        self.inner.write_u8(value);
    }

    pub(crate) fn write_u16(&mut self, value: u16) {
        self.inner.write_u16(value);
    }

    pub(crate) fn write_u32(&mut self, value: u32) {
        self.inner.write_u32(value);
    }

    pub(crate) fn write_i32(&mut self, value: i32) {
        self.inner.write_i32(value);
    }

    pub(crate) fn write_u64(&mut self, value: u64) {
        self.inner.write_u64(value);
    }

    pub(crate) fn write_f32(&mut self, value: f32) {
        self.inner.write_f32(value);
    }

    pub(crate) fn write_counted_str(
        &mut self,
        what: &'static str,
        value: &str,
    ) -> Result<(), PexError> {
        if value.len() > u16::MAX as usize {
            return Err(PexError::StringTooLong {
                what,
                len: value.len(),
            });
        }
        self.write_u16(value.len() as u16);
        self.inner.extend(value.as_bytes());
        Ok(())
    }

    pub(crate) fn write_string_id(
        &mut self,
        id: PexStringId,
        table_len: usize,
    ) -> Result<(), PexError> {
        if id.index() as usize >= table_len {
            return Err(PexError::WriteStringIdOutOfRange { id, table_len });
        }
        self.write_u16(id.index());
        Ok(())
    }
}

pub(crate) struct BinaryReader<'a> {
    inner: CoreBinaryReader<'a>,
}

impl<'a> BinaryReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: CoreBinaryReader::new(bytes, Endian::Big),
        }
    }

    pub(crate) const fn offset(&self) -> usize {
        self.inner.offset()
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    pub(crate) fn read_magic(&mut self) -> Result<u32, PexError> {
        let bytes = self.take(4, "magic")?;
        if bytes == PEX_MAGIC.to_be_bytes() {
            self.inner.set_endian(Endian::Big);
            Ok(PEX_MAGIC)
        } else if bytes == PEX_MAGIC.to_le_bytes() {
            self.inner.set_endian(Endian::Little);
            Ok(PEX_MAGIC)
        } else {
            Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
    }

    pub(crate) fn read_u8(&mut self, what: &'static str) -> Result<u8, PexError> {
        self.inner.read_u8(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_u16(&mut self, what: &'static str) -> Result<u16, PexError> {
        self.inner.read_u16(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_i32(&mut self, what: &'static str) -> Result<i32, PexError> {
        self.inner.read_i32(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_u32(&mut self, what: &'static str) -> Result<u32, PexError> {
        self.inner.read_u32(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_u64(&mut self, what: &'static str) -> Result<u64, PexError> {
        self.inner.read_u64(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_f32(&mut self, what: &'static str) -> Result<f32, PexError> {
        self.inner.read_f32(what).map_err(pex_binary_error)
    }

    pub(crate) fn read_counted_str(&mut self, what: &'static str) -> Result<String, PexError> {
        let len = usize::from(self.read_u16(what)?);
        let offset = self.offset();
        let bytes = self.take(len, what)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| PexError::InvalidUtf8 { offset, what })
    }

    pub(crate) fn read_string_id(
        &mut self,
        table_len: usize,
        what: &'static str,
    ) -> Result<PexStringId, PexError> {
        let id = PexStringId::new(self.read_u16(what)?);
        if id.index() as usize >= table_len {
            return Err(PexError::StringIdOutOfRange {
                id,
                table_len,
                what,
            });
        }
        Ok(id)
    }

    pub(crate) fn take(&mut self, len: usize, what: &'static str) -> Result<&'a [u8], PexError> {
        self.inner.take(len, what).map_err(pex_binary_error)
    }
}

fn pex_binary_error(error: BinaryError) -> PexError {
    match error {
        BinaryError::Truncated {
            offset,
            needed,
            remaining,
            what,
        } => PexError::Truncated {
            offset,
            needed,
            remaining,
            what,
        },
        BinaryError::InvalidLength { offset, len, what } => PexError::Truncated {
            offset,
            needed: len.max(0) as usize,
            remaining: 0,
            what,
        },
    }
}
