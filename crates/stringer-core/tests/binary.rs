use stringer_core::binary::{BinaryError, BinaryReader, BinaryWriter, Endian, read_u32_at};

#[test]
fn reader_reports_truncated_reads_with_offsets() {
    let mut reader = BinaryReader::new(&[0xAA, 0xBB], Endian::Little);

    let error = reader.read_u32("record length").unwrap_err();

    assert_eq!(
        error,
        BinaryError::Truncated {
            offset: 0,
            needed: 4,
            remaining: 2,
            what: "record length",
        }
    );
    assert_eq!(reader.offset(), 0);
}

#[test]
fn reader_respects_little_and_big_endian_numbers() {
    let bytes = [0x01, 0x02, 0x03, 0x04];

    let mut little = BinaryReader::new(&bytes, Endian::Little);
    let mut big = BinaryReader::new(&bytes, Endian::Big);

    assert_eq!(little.read_u32("little value").unwrap(), 0x0403_0201);
    assert_eq!(big.read_u32("big value").unwrap(), 0x0102_0304);
}

#[test]
fn writer_output_round_trips_through_reader() {
    let mut writer = BinaryWriter::new(Endian::Big);
    writer.write_u8(0xFE);
    writer.write_u16(0x0102);
    writer.write_i32(-7);
    writer.write_f32(1.5);
    writer.extend([0xAA, 0xBB]);

    let bytes = writer.into_bytes();
    let mut reader = BinaryReader::new(&bytes, Endian::Big);

    assert_eq!(reader.read_u8("tag").unwrap(), 0xFE);
    assert_eq!(reader.read_u16("count").unwrap(), 0x0102);
    assert_eq!(reader.read_i32("signed").unwrap(), -7);
    assert_eq!(reader.read_f32("float").unwrap(), 1.5);
    assert_eq!(reader.take(2, "tail").unwrap(), &[0xAA, 0xBB]);
    assert_eq!(reader.remaining(), 0);
}

#[test]
fn offset_helpers_report_truncated_offsets() {
    let bytes = [0x00, 0x01, 0x02];

    let error = read_u32_at(&bytes, 1, Endian::Little, "directory offset").unwrap_err();

    assert_eq!(
        error,
        BinaryError::Truncated {
            offset: 1,
            needed: 4,
            remaining: 2,
            what: "directory offset",
        }
    );
}
