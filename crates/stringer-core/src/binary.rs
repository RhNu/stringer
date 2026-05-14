use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BinaryError {
    #[error("truncated {what} at byte {offset}: needed {needed} bytes, only {remaining} remaining")]
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
        what: &'static str,
    },
    #[error("invalid {what} length {len} at byte {offset}")]
    InvalidLength {
        offset: usize,
        len: i64,
        what: &'static str,
    },
}

pub fn read_u16_at(
    bytes: &[u8],
    offset: usize,
    endian: Endian,
    what: &'static str,
) -> Result<u16, BinaryError> {
    let bytes = take_at(bytes, offset, 2, what)?;
    Ok(match endian {
        Endian::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
        Endian::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
    })
}

pub fn read_u32_at(
    bytes: &[u8],
    offset: usize,
    endian: Endian,
    what: &'static str,
) -> Result<u32, BinaryError> {
    let bytes = take_at(bytes, offset, 4, what)?;
    Ok(match endian {
        Endian::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        Endian::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
    })
}

fn take_at<'a>(
    bytes: &'a [u8],
    offset: usize,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8], BinaryError> {
    let remaining = bytes.len().saturating_sub(offset);
    if remaining < len {
        return Err(BinaryError::Truncated {
            offset,
            needed: len,
            remaining,
            what,
        });
    }
    Ok(&bytes[offset..offset + len])
}

#[derive(Debug, Clone)]
pub struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    endian: Endian,
}

impl<'a> BinaryReader<'a> {
    pub fn new(bytes: &'a [u8], endian: Endian) -> Self {
        Self {
            bytes,
            offset: 0,
            endian,
        }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn endian(&self) -> Endian {
        self.endian
    }

    pub fn set_endian(&mut self, endian: Endian) {
        self.endian = endian;
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn take(&mut self, len: usize, what: &'static str) -> Result<&'a [u8], BinaryError> {
        let offset = self.offset;
        let remaining = self.remaining();
        if remaining < len {
            return Err(BinaryError::Truncated {
                offset,
                needed: len,
                remaining,
                what,
            });
        }
        self.offset += len;
        Ok(&self.bytes[offset..offset + len])
    }

    pub fn read_u8(&mut self, what: &'static str) -> Result<u8, BinaryError> {
        Ok(self.take(1, what)?[0])
    }

    pub fn read_i16(&mut self, what: &'static str) -> Result<i16, BinaryError> {
        let bytes = self.take(2, what)?;
        Ok(match self.endian {
            Endian::Little => i16::from_le_bytes([bytes[0], bytes[1]]),
            Endian::Big => i16::from_be_bytes([bytes[0], bytes[1]]),
        })
    }

    pub fn read_u16(&mut self, what: &'static str) -> Result<u16, BinaryError> {
        let bytes = self.take(2, what)?;
        Ok(match self.endian {
            Endian::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
            Endian::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
        })
    }

    pub fn read_i32(&mut self, what: &'static str) -> Result<i32, BinaryError> {
        let bytes = self.take(4, what)?;
        Ok(match self.endian {
            Endian::Little => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endian::Big => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn read_u32(&mut self, what: &'static str) -> Result<u32, BinaryError> {
        let bytes = self.take(4, what)?;
        Ok(match self.endian {
            Endian::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endian::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn read_u64(&mut self, what: &'static str) -> Result<u64, BinaryError> {
        let bytes = self.take(8, what)?;
        Ok(match self.endian {
            Endian::Little => u64::from_le_bytes(bytes.try_into().expect("u64 slice")),
            Endian::Big => u64::from_be_bytes(bytes.try_into().expect("u64 slice")),
        })
    }

    pub fn read_f32(&mut self, what: &'static str) -> Result<f32, BinaryError> {
        let bytes = self.take(4, what)?;
        Ok(match self.endian {
            Endian::Little => f32::from_le_bytes(bytes.try_into().expect("f32 slice")),
            Endian::Big => f32::from_be_bytes(bytes.try_into().expect("f32 slice")),
        })
    }
}

#[derive(Debug, Clone)]
pub struct BinaryWriter {
    endian: Endian,
    bytes: Vec<u8>,
}

impl BinaryWriter {
    pub fn new(endian: Endian) -> Self {
        Self {
            endian,
            bytes: Vec::new(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn extend(&mut self, bytes: impl AsRef<[u8]>) {
        self.bytes.extend(bytes.as_ref());
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        match self.endian {
            Endian::Little => self.bytes.extend(value.to_le_bytes()),
            Endian::Big => self.bytes.extend(value.to_be_bytes()),
        }
    }

    pub fn write_u32(&mut self, value: u32) {
        match self.endian {
            Endian::Little => self.bytes.extend(value.to_le_bytes()),
            Endian::Big => self.bytes.extend(value.to_be_bytes()),
        }
    }

    pub fn write_i32(&mut self, value: i32) {
        match self.endian {
            Endian::Little => self.bytes.extend(value.to_le_bytes()),
            Endian::Big => self.bytes.extend(value.to_be_bytes()),
        }
    }

    pub fn write_u64(&mut self, value: u64) {
        match self.endian {
            Endian::Little => self.bytes.extend(value.to_le_bytes()),
            Endian::Big => self.bytes.extend(value.to_be_bytes()),
        }
    }

    pub fn write_f32(&mut self, value: f32) {
        match self.endian {
            Endian::Little => self.bytes.extend(value.to_le_bytes()),
            Endian::Big => self.bytes.extend(value.to_be_bytes()),
        }
    }
}
