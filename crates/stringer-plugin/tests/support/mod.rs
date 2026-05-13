#![allow(dead_code)]

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use std::io::{Read, Write};

pub const COMPRESSED_FLAG: u32 = 0x0004_0000;
pub const LOCALIZED_FLAG: u32 = 0x0000_0080;

pub struct FixtureRecord {
    pub record_type: &'static str,
    pub form_id: u32,
    pub flags: u32,
    pub subrecords: Vec<Vec<u8>>,
}

pub fn localized_header() -> Vec<u8> {
    vec![LOCALIZED_FLAG as u8, 0, 0, 0]
}

pub fn build_plugin(tes4_content: Vec<u8>, top_level: Vec<Vec<u8>>) -> Vec<u8> {
    build_plugin_with_flags(LOCALIZED_FLAG, tes4_content, top_level)
}

pub fn build_plugin_with_flags(
    tes4_flags: u32,
    tes4_content: Vec<u8>,
    top_level: Vec<Vec<u8>>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(build_major(
        "TES4",
        0,
        tes4_flags,
        vec![build_subrecord("HEDR", &tes4_content)],
    ));
    for item in top_level {
        bytes.extend(item);
    }
    bytes
}

pub fn build_group(label: &str, records: Vec<Vec<u8>>) -> Vec<u8> {
    let mut content = Vec::new();
    for record in records {
        content.extend(record);
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GRUP");
    bytes.extend_from_slice(&(24u32 + content.len() as u32).to_le_bytes());
    let mut label_bytes = [0u8; 4];
    label_bytes.copy_from_slice(label.as_bytes());
    bytes.extend_from_slice(&label_bytes);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend(content);
    bytes
}

pub fn build_major(
    record_type: &str,
    form_id: u32,
    flags: u32,
    subrecords: Vec<Vec<u8>>,
) -> Vec<u8> {
    let mut content = Vec::new();
    for subrecord in subrecords {
        content.extend(subrecord);
    }
    build_major_from_content(record_type, form_id, flags, content)
}

pub fn build_major_from_content(
    record_type: &str,
    form_id: u32,
    flags: u32,
    content: Vec<u8>,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(record_type.as_bytes());
    bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&form_id.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&44u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend(content);
    bytes
}

pub fn compressed_record(record: FixtureRecord) -> Vec<u8> {
    let mut content = Vec::new();
    for subrecord in record.subrecords {
        content.extend(subrecord);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut compressed_content = Vec::new();
    compressed_content.extend_from_slice(&(content.len() as u32).to_le_bytes());
    compressed_content.extend_from_slice(&compressed);
    build_major_from_content(
        record.record_type,
        record.form_id,
        record.flags | COMPRESSED_FLAG,
        compressed_content,
    )
}

pub fn compressed_record_with_declared_len(record: FixtureRecord, declared_len: u32) -> Vec<u8> {
    let mut content = Vec::new();
    for subrecord in record.subrecords {
        content.extend(subrecord);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content).unwrap();
    let compressed = encoder.finish().unwrap();

    let mut compressed_content = Vec::new();
    compressed_content.extend_from_slice(&declared_len.to_le_bytes());
    compressed_content.extend_from_slice(&compressed);
    build_major_from_content(
        record.record_type,
        record.form_id,
        record.flags | COMPRESSED_FLAG,
        compressed_content,
    )
}

pub fn build_subrecord(record_type: &str, content: &[u8]) -> Vec<u8> {
    assert!(content.len() <= u16::MAX as usize);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(record_type.as_bytes());
    bytes.extend_from_slice(&(content.len() as u16).to_le_bytes());
    bytes.extend_from_slice(content);
    bytes
}

pub fn xxxx_subrecord(record_type: &str, content: &[u8]) -> Vec<u8> {
    assert!(content.len() > u16::MAX as usize);
    let mut bytes = Vec::new();
    bytes.extend(build_subrecord(
        "XXXX",
        &(content.len() as u32).to_le_bytes(),
    ));
    bytes.extend_from_slice(record_type.as_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(content);
    bytes
}

pub fn decompress_major_content(bytes: &[u8]) -> Vec<u8> {
    let len = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    let mut decoder = ZlibDecoder::new(&bytes[28..]);
    let mut output = Vec::with_capacity(len);
    decoder.read_to_end(&mut output).unwrap();
    output
}
