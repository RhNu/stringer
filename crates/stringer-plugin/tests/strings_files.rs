use bytes::Bytes;
use stringer_core::FileAsset;
use stringer_plugin::{
    GameRelease, Language, PluginError, StringsFile, StringsKind, parse_strings_file,
    write_strings_file,
};

#[test]
fn roundtrips_normal_strings_with_null_terminated_data() {
    let mut file = StringsFile::new(StringsKind::Normal, Language::English);
    file.insert(42, "Iron Sword");
    file.insert(7, "Steel Armor");

    let asset = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &file,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    let parsed = parse_strings_file(&asset, GameRelease::SkyrimSe, Language::English).unwrap();

    assert_eq!(parsed.kind(), StringsKind::Normal);
    assert_eq!(parsed.get(42), Some("Iron Sword"));
    assert_eq!(parsed.get(7), Some("Steel Armor"));
}

#[test]
fn roundtrips_dlstrings_and_ilstrings_with_length_prefixed_data() {
    for kind in [StringsKind::Dl, StringsKind::Il] {
        let mut file = StringsFile::new(kind, Language::French);
        file.insert(1, "Une description longue");
        file.insert(2, "Une ligne de dialogue");

        let asset = write_strings_file(
            format!("Data/Strings/MyMod_French.{}", kind.extension()),
            &file,
            GameRelease::SkyrimSe,
        )
        .unwrap();
        let parsed = parse_strings_file(&asset, GameRelease::SkyrimSe, Language::French).unwrap();

        assert_eq!(parsed.kind(), kind);
        assert_eq!(parsed.get(1), Some("Une description longue"));
        assert_eq!(parsed.get(2), Some("Une ligne de dialogue"));
    }
}

#[test]
fn rejects_duplicate_string_ids() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(b"A\0B\0");
    let asset = FileAsset::new("Data/Strings/MyMod_English.STRINGS", Bytes::from(bytes));

    let error = parse_strings_file(&asset, GameRelease::SkyrimSe, Language::English).unwrap_err();

    assert!(matches!(
        error,
        PluginError::DuplicateStringId { id: 1, .. }
    ));
}

#[test]
fn rejects_string_offsets_outside_the_data_section() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&99u32.to_le_bytes());
    bytes.extend_from_slice(b"A\0");
    let asset = FileAsset::new("Data/Strings/MyMod_English.STRINGS", Bytes::from(bytes));

    let error = parse_strings_file(&asset, GameRelease::SkyrimSe, Language::English).unwrap_err();

    assert!(matches!(error, PluginError::MalformedStrings { .. }));
}

#[test]
fn decodes_skyrim_le_legacy_code_pages() {
    let mut file = StringsFile::new(StringsKind::Normal, Language::Russian);
    file.insert(10, "Привет");

    let asset = write_strings_file(
        "Data/Strings/MyMod_Russian.STRINGS",
        &file,
        GameRelease::SkyrimLe,
    )
    .unwrap();
    let parsed = parse_strings_file(&asset, GameRelease::SkyrimLe, Language::Russian).unwrap();

    assert_eq!(parsed.get(10), Some("Привет"));
}
