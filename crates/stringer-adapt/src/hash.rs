use std::{collections::BTreeMap, hash::Hasher};

pub(crate) fn adapt_id(
    format: &str,
    source: &str,
    target: &str,
    context: &BTreeMap<String, String>,
) -> String {
    let mut hasher = Fnv64::default();
    hasher.write(format.as_bytes());
    hasher.write_u8(0);
    hasher.write(source.as_bytes());
    hasher.write_u8(0);
    hasher.write(target.as_bytes());
    for (key, value) in context {
        hasher.write_u8(0);
        hasher.write(key.as_bytes());
        hasher.write_u8(b'=');
        hasher.write(value.as_bytes());
    }
    format!("adapt:{format}:{:016x}", hasher.finish())
}

#[derive(Default)]
struct Fnv64(u64);

impl Hasher for Fnv64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf2_9ce4_8422_2325;
        }
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}
