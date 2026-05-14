use stringer_core::binary::BinaryReader;

pub(crate) trait AdaptBinaryReaderExt<'a> {
    fn read_ascii(&mut self, count: usize, what: &'static str) -> Result<String, String>;
    fn read_utf8_u16_string(&mut self, what: &'static str) -> Result<String, String>;
    fn read_utf8_u32_string(&mut self, what: &'static str) -> Result<String, String>;
    fn read_utf16_i32_string(&mut self, what: &'static str) -> Result<String, String>;
}

impl<'a> AdaptBinaryReaderExt<'a> for BinaryReader<'a> {
    fn read_ascii(&mut self, count: usize, what: &'static str) -> Result<String, String> {
        let bytes = self.take(count, what).map_err(|error| error.to_string())?;
        Ok(String::from_utf8_lossy(bytes).to_string())
    }

    fn read_utf8_u16_string(&mut self, what: &'static str) -> Result<String, String> {
        let len = usize::from(self.read_u16(what).map_err(|error| error.to_string())?);
        let bytes = self.take(len, what).map_err(|error| error.to_string())?;
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    }

    fn read_utf8_u32_string(&mut self, what: &'static str) -> Result<String, String> {
        let len = self.read_u32(what).map_err(|error| error.to_string())? as usize;
        let bytes = self.take(len, what).map_err(|error| error.to_string())?;
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    }

    fn read_utf16_i32_string(&mut self, what: &'static str) -> Result<String, String> {
        let len = self.read_i32(what).map_err(|error| error.to_string())?;
        if len < 0 || len % 2 != 0 {
            return Err(format!("invalid UTF-16 byte length {len}"));
        }
        let bytes = self
            .take(len as usize, what)
            .map_err(|error| error.to_string())?;
        let words = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&words).map_err(|error| error.to_string())
    }
}
