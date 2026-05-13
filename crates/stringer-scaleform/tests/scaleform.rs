use bytes::{Bytes, BytesMut};
use stringer_core::{
    FileAsset, FileBundle, Language, ScaleformStringMetadata, StringEntryBundle, StringEntrySource,
};
use stringer_scaleform::{
    ScaleformError, ScaleformTranslationFile, parse_scaleform_translation_file,
    read_scaleform_translations, write_scaleform_translation_file, write_scaleform_translations,
};

#[test]
fn roundtrips_utf16le_bom_scaleform_tsv() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        utf16le_bom("$Title\tIron Sword\r\n$Desc\tSharp blade\r\n"),
    );

    let file = parse_scaleform_translation_file(&asset).unwrap();
    let output = write_scaleform_translation_file(&file).unwrap();

    assert_eq!(file.get("$Title"), Some("Iron Sword"));
    assert_eq!(file.get("$Desc"), Some("Sharp blade"));
    assert_eq!(output.bytes(), asset.bytes());
}

#[test]
fn reads_utf8_bom_and_writes_utf16le_after_changes() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        Bytes::from_static(b"\xEF\xBB\xBF$Title\tIron Sword\n"),
    );
    let mut file = parse_scaleform_translation_file(&asset).unwrap();

    file.entry_mut("$Title").unwrap().set_text("Steel Sword");
    let output = write_scaleform_translation_file(&file).unwrap();

    assert!(output.bytes().starts_with(&[0xFF, 0xFE]));
    let reparsed = parse_scaleform_translation_file(&output).unwrap();
    assert_eq!(reparsed.get("$Title"), Some("Steel Sword"));
}

#[test]
fn preserves_blank_comments_and_entry_order() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        utf16le_bom("# Header\r\n\r\n$First\tOne\r\n; Comment\r\n$Second\tTwo\r\n"),
    );
    let mut file = parse_scaleform_translation_file(&asset).unwrap();

    file.entry_mut("$Second").unwrap().set_text("Deux");
    let output = write_scaleform_translation_file(&file).unwrap();
    let text = decode_utf16le_bom(output.bytes());

    assert_eq!(
        text,
        "# Header\r\n\r\n$First\tOne\r\n; Comment\r\n$Second\tDeux\r\n"
    );
}

#[test]
fn rejects_malformed_rows_and_duplicate_keys() {
    for (body, expected_line) in [
        ("$One\tFirst\n$One\tSecond\n", 2),
        ("NoTabHere\n", 1),
        ("Key\tValue\n", 1),
    ] {
        let asset = FileAsset::new(
            "Data/Interface/Translations/MyMod_ENGLISH.txt",
            Bytes::from(body.as_bytes().to_vec()),
        );

        let error = parse_scaleform_translation_file(&asset).unwrap_err();

        assert_eq!(error.line(), Some(expected_line));
        assert!(matches!(
            error,
            ScaleformError::DuplicateKey { .. } | ScaleformError::MalformedRow { .. }
        ));
    }
}

#[test]
fn rejects_invalid_insert_keys() {
    let mut file = ScaleformTranslationFile::new("Data/Interface/Translations/MyMod_ENGLISH.txt");

    for key in ["$Bad\tKey", "$Bad\nKey"] {
        let error = file.insert(key, "Value").unwrap_err();

        assert!(matches!(error, ScaleformError::InvalidKey { .. }));
    }
}

#[test]
fn rejects_parsed_keys_with_carriage_returns() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        Bytes::from_static(b"$Bad\rKey\tValue\n"),
    );

    let error = parse_scaleform_translation_file(&asset).unwrap_err();

    assert!(matches!(error, ScaleformError::MalformedRow { .. }));
    assert_eq!(error.line(), Some(1));
}

#[test]
fn rejects_written_values_that_would_create_extra_rows() {
    let mut file = ScaleformTranslationFile::new("Data/Interface/Translations/MyMod_ENGLISH.txt");
    file.insert("$Title", "Iron Sword").unwrap();
    file.entry_mut("$Title").unwrap().set_text("Iron\nSword");

    let error = write_scaleform_translation_file(&file).unwrap_err();

    assert!(matches!(error, ScaleformError::InvalidText { .. }));
}

#[test]
fn writes_reordered_entries_to_their_original_keys() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        utf16le_bom("$First\tOne\r\n$Second\tTwo\r\n"),
    );
    let bundle = FileBundle::new(vec![asset]);
    let mut translations = read_scaleform_translations(bundle, Language::English).unwrap();

    translations.string_entries_mut().swap(0, 1);
    translations.string_entries_mut()[0].set_text("Deux");
    let output = write_scaleform_translations(translations).unwrap();
    let reparsed = parse_scaleform_translation_file(
        output
            .get("Data/Interface/Translations/MyMod_ENGLISH.txt")
            .unwrap(),
    )
    .unwrap();

    assert_eq!(reparsed.get("$First"), Some("One"));
    assert_eq!(reparsed.get("$Second"), Some("Deux"));
}

#[test]
fn new_file_insert_writes_valid_scaleform_translation() {
    let mut file = ScaleformTranslationFile::new("Data/Interface/Translations/MyMod_ENGLISH.txt");
    file.insert("$Title", "Iron Sword").unwrap();

    let output = write_scaleform_translation_file(&file).unwrap();
    let reparsed = parse_scaleform_translation_file(&output).unwrap();

    assert_eq!(reparsed.get("$Title"), Some("Iron Sword"));
    assert!(decode_utf16le_bom(output.bytes()).contains("$Title\tIron Sword\r\n"));
}

#[test]
fn high_level_entries_expose_scaleform_metadata() {
    let asset = FileAsset::new(
        "Data/Interface/Translations/MyMod_ENGLISH.txt",
        utf16le_bom("$Title\tIron Sword\r\n"),
    );
    let bundle = FileBundle::new(vec![asset]);

    let translations = read_scaleform_translations(bundle, Language::English).unwrap();

    let StringEntrySource::Scaleform(ScaleformStringMetadata { path, key }) =
        translations.string_entries()[0].source()
    else {
        panic!("expected scaleform metadata");
    };
    assert_eq!(
        path.as_str(),
        "Data/Interface/Translations/MyMod_ENGLISH.txt"
    );
    assert_eq!(key.as_deref(), Some("$Title"));
}

fn utf16le_bom(text: &str) -> Bytes {
    let mut bytes = BytesMut::from(&[0xFF, 0xFE][..]);
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes.freeze()
}

fn decode_utf16le_bom(bytes: &Bytes) -> String {
    assert!(bytes.starts_with(&[0xFF, 0xFE]));
    let units = bytes[2..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).unwrap()
}
