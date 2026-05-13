mod support;

use bytes::Bytes;
use stringer_core::FileAsset;
use stringer_plugin::{
    GameRelease, LocalizedFieldSource, ParsePluginOptions, PluginError, StringsKind,
    parse_plugin_file, write_plugin_file,
};
use support::{
    FixtureRecord, build_group, build_major, build_plugin, build_plugin_with_flags,
    build_subrecord, compressed_record, compressed_record_with_declared_len, localized_header,
    xxxx_subrecord,
};

#[test]
fn extracts_localized_skyrim_fields_using_the_field_registry() {
    let plugin = build_plugin(
        localized_header(),
        vec![
            build_major(
                "WEAP",
                0x800,
                0,
                vec![build_subrecord("FULL", &42u32.to_le_bytes())],
            ),
            build_major(
                "DIAL",
                0x801,
                0,
                vec![build_subrecord("FULL", &9u32.to_le_bytes())],
            ),
            build_major(
                "INFO",
                0x802,
                0,
                vec![build_subrecord("NAM1", &77u32.to_le_bytes())],
            ),
        ],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert!(parsed.is_localized());
    assert_eq!(parsed.entries().len(), 3);
    assert_eq!(parsed.entries()[0].source(), LocalizedFieldSource::Normal);
    assert_eq!(parsed.entries()[1].source(), LocalizedFieldSource::Normal);
    assert_eq!(parsed.entries()[2].source(), LocalizedFieldSource::Il);
    assert_eq!(parsed.entries()[2].strings_kind(), StringsKind::Il);
}

#[test]
fn distinguishes_quest_nnam_description_from_objective_display_text() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "QUST",
            0x900,
            0,
            vec![
                build_subrecord("NNAM", &1u32.to_le_bytes()),
                build_subrecord("QOBJ", &10u16.to_le_bytes()),
                build_subrecord("FNAM", &0u32.to_le_bytes()),
                build_subrecord("NNAM", &2u32.to_le_bytes()),
            ],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert_eq!(parsed.entries().len(), 2);
    assert_eq!(parsed.entries()[0].source(), LocalizedFieldSource::Dl);
    assert_eq!(parsed.entries()[1].source(), LocalizedFieldSource::Normal);
}

#[test]
fn only_treats_string_game_settings_data_as_localized() {
    let plugin = build_plugin(
        localized_header(),
        vec![
            build_major(
                "GMST",
                0x901,
                0,
                vec![
                    build_subrecord("EDID", b"sStringSetting\0"),
                    build_subrecord("DATA", &9u32.to_le_bytes()),
                ],
            ),
            build_major(
                "GMST",
                0x902,
                0,
                vec![
                    build_subrecord("EDID", b"fFloatSetting\0"),
                    build_subrecord("DATA", &1u32.to_le_bytes()),
                ],
            ),
        ],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert_eq!(parsed.entries().len(), 1);
    assert_eq!(parsed.entries()[0].form_id(), 0x901);
    assert_eq!(parsed.entries()[0].string_id(), Some(9));
}

#[test]
fn does_not_treat_non_text_perk_epft_payload_as_localized() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "PERK",
            0x903,
            0,
            vec![build_subrecord("EPFT", &[1, 2, 3, 4])],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert!(parsed.entries().is_empty());
}

#[test]
fn rejects_embedded_text_on_localized_plugin_entries_instead_of_writing_zero_id() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &42u32.to_le_bytes())],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));
    let mut parsed =
        parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    parsed.entries_mut()[0].set_embedded_text("Steel Sword");
    let error = write_plugin_file(&parsed).unwrap_err();

    assert!(matches!(
        error,
        PluginError::InvalidLocalizedEntryState { .. }
    ));
}

#[test]
fn rejects_compressed_records_with_unreasonable_declared_lengths() {
    let record = compressed_record_with_declared_len(
        FixtureRecord {
            record_type: "WEAP",
            form_id: 0x804,
            flags: 0,
            subrecords: vec![build_subrecord("FULL", &42u32.to_le_bytes())],
        },
        0xFFFF_FFFF,
    );
    let plugin = build_plugin(localized_header(), vec![record]);
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let error =
        parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap_err();

    assert!(matches!(error, PluginError::MalformedPlugin { .. }));
}

#[test]
fn preserves_unknown_subrecords_when_writing_modified_localized_fields() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![
                build_subrecord("EDID", b"WeaponEditorId\0"),
                build_subrecord("FULL", &42u32.to_le_bytes()),
                build_subrecord("DATA", &[1, 2, 3, 4, 5, 6]),
            ],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin.clone()));
    let mut parsed =
        parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();
    parsed.entries_mut()[0].set_string_id(43);

    let written = write_plugin_file(&parsed).unwrap();
    let reparsed =
        parse_plugin_file(&written, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert_eq!(reparsed.entries()[0].string_id(), Some(43));
    assert!(
        written
            .bytes()
            .windows(b"WeaponEditorId\0".len())
            .any(|w| w == b"WeaponEditorId\0")
    );
    assert!(
        written
            .bytes()
            .windows([1, 2, 3, 4, 5, 6].len())
            .any(|w| w == [1, 2, 3, 4, 5, 6])
    );
    assert_ne!(written.bytes().as_ref(), plugin.as_slice());
}

#[test]
fn keeps_unmodified_plugin_bytes_identical_after_roundtrip() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_group(
            "WEAP",
            vec![build_major(
                "WEAP",
                0x800,
                0,
                vec![build_subrecord("FULL", &42u32.to_le_bytes())],
            )],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin.clone()));
    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    let written = write_plugin_file(&parsed).unwrap();

    assert_eq!(written.bytes().as_ref(), plugin.as_slice());
}

#[test]
fn handles_xxxx_overflow_subrecords() {
    let long_text = vec![b'A'; 70_000];
    let plugin = build_plugin_with_flags(
        0,
        Vec::new(),
        vec![build_major(
            "BOOK",
            0x900,
            0,
            vec![xxxx_subrecord("DESC", &long_text)],
        )],
    );
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let parsed = parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert_eq!(parsed.entries().len(), 1);
    assert_eq!(
        parsed.entries()[0].embedded_text(),
        Some("A".repeat(70_000).as_str())
    );
}

#[test]
fn extracts_fields_inside_compressed_major_records_and_recompresses_on_write() {
    let record = compressed_record(FixtureRecord {
        record_type: "WEAP",
        form_id: 0x800,
        flags: 0,
        subrecords: vec![build_subrecord("FULL", &42u32.to_le_bytes())],
    });
    let plugin = build_plugin(localized_header(), vec![record]);
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::from(plugin));

    let mut parsed =
        parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();
    parsed.entries_mut()[0].set_string_id(43);
    let written = write_plugin_file(&parsed).unwrap();
    let reparsed =
        parse_plugin_file(&written, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap();

    assert_eq!(reparsed.entries()[0].string_id(), Some(43));
    assert!(reparsed.records()[0].is_compressed());
}

#[test]
fn rejects_truncated_major_records() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major("WEAP", 0x800, 0, vec![])],
    );
    let truncated = &plugin[..plugin.len() - 3];
    let asset = FileAsset::new("Data/MyMod.esp", Bytes::copy_from_slice(truncated));

    let error =
        parse_plugin_file(&asset, ParsePluginOptions::new(GameRelease::SkyrimSe)).unwrap_err();

    assert!(matches!(error, PluginError::MalformedPlugin { .. }));
}
