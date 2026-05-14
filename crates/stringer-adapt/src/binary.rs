pub(crate) struct BinaryReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BinaryReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .pos
            .checked_add(count)
            .ok_or_else(|| "read offset overflow".to_string())?;
        if end > self.bytes.len() {
            return Err(format!(
                "unexpected end of input at byte {}, need {} bytes",
                self.pos, count
            ));
        }
        let value = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(value)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_bytes(1)?[0])
    }

    pub(crate) fn read_i16(&mut self) -> Result<i16, String> {
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(self.read_bytes(2)?);
        Ok(i16::from_le_bytes(bytes))
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, String> {
        let mut bytes = [0u8; 2];
        bytes.copy_from_slice(self.read_bytes(2)?);
        Ok(u16::from_le_bytes(bytes))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32, String> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.read_bytes(4)?);
        Ok(i32::from_le_bytes(bytes))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32, String> {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.read_bytes(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    pub(crate) fn read_ascii(&mut self, count: usize) -> Result<String, String> {
        let bytes = self.read_bytes(count)?;
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    pub(crate) fn read_utf8_u16_string(&mut self) -> Result<String, String> {
        let len = usize::from(self.read_u16()?);
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    }

    pub(crate) fn read_utf8_u32_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    }

    pub(crate) fn read_utf16_i32_string(&mut self) -> Result<String, String> {
        let len = self.read_i32()?;
        if len < 0 || len % 2 != 0 {
            return Err(format!("invalid UTF-16 byte length {len}"));
        }
        let bytes = self.read_bytes(len as usize)?;
        let words = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&words).map_err(|error| error.to_string())
    }
}
