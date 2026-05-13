mod support;

use bytes::Bytes;
use stringer_core::{FileAsset, FileBundle, PluginStringStorage, StringEntrySource};
use stringer_plugin::{
    GameRelease, Language, PluginError, ReadOptions, StringsFile, StringsKind, WriteOptions,
    read_localization, write_localization, write_strings_file,
};
use support::{
    build_major, build_plugin, build_plugin_with_flags, build_subrecord, localized_header,
};

#[tokio::test]
async fn reads_and_writes_localization_bundle_from_file_abstractions() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let strings_asset = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &strings,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        strings_asset,
    ]);

    let mut localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();
    localization.entries_mut()[0].set_text("Steel Sword");

    let output = write_localization(
        localization,
        WriteOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    let strings_output = output
        .get("Data/Strings/MyMod_English.STRINGS")
        .expect("strings output");
    let plugin_output = output.get("Data/MyMod.esp").expect("plugin output");

    assert!(plugin_output.bytes().len() > 24);
    assert!(
        strings_output
            .bytes()
            .windows(b"Steel Sword\0".len())
            .any(|w| w == b"Steel Sword\0")
    );
}

#[tokio::test]
async fn preserves_unmodified_bundle_bytes() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let strings_asset = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &strings,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin.clone())),
        strings_asset.clone(),
    ]);

    let localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();
    let output = write_localization(
        localization,
        WriteOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    assert_eq!(
        output.get("Data/MyMod.esp").unwrap().bytes().as_ref(),
        plugin.as_slice()
    );
    assert_eq!(
        output
            .get("Data/Strings/MyMod_English.STRINGS")
            .unwrap()
            .bytes(),
        strings_asset.bytes()
    );
}

#[tokio::test]
async fn ignores_strings_files_for_other_mods_and_languages() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    );
    let mut mymod_english = StringsFile::new(StringsKind::Normal, Language::English);
    mymod_english.insert(1, "Iron Sword");
    let mut other_english = StringsFile::new(StringsKind::Normal, Language::English);
    other_english.insert(1, "Wrong Mod");
    let mut mymod_french = StringsFile::new(StringsKind::Normal, Language::French);
    mymod_french.insert(1, "Mauvaise langue");
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        write_strings_file(
            "Data/Strings/MyMod_English.STRINGS",
            &mymod_english,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
        write_strings_file(
            "Data/Strings/OtherMod_English.STRINGS",
            &other_english,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
        write_strings_file(
            "Data/Strings/MyMod_French.STRINGS",
            &mymod_french,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
    ]);

    let localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    assert_eq!(localization.entries()[0].text(), "Iron Sword");
}

#[tokio::test]
async fn exposes_plugin_metadata_on_high_level_entries() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &42u32.to_le_bytes())],
        )],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(42, "Iron Sword");
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        write_strings_file(
            "Data/Strings/MyMod_English.STRINGS",
            &strings,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
    ]);

    let localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    let StringEntrySource::Plugin(metadata) = localization.entries()[0].source() else {
        panic!("expected plugin string metadata");
    };
    assert_eq!(
        localization.entries()[0].strings_kind(),
        StringsKind::Normal
    );
    assert_eq!(localization.entries()[0].string_id(), Some(42));
    assert_eq!(metadata.path.as_str(), "Data/MyMod.esp");
    assert_eq!(metadata.record_type, "WEAP");
    assert_eq!(metadata.form_id, 0x800);
    assert_eq!(metadata.subrecord, "FULL");
    assert_eq!(metadata.strings_kind, "STRINGS");
    assert_eq!(metadata.field_source, "Normal");
    assert_eq!(metadata.storage, PluginStringStorage::Localized);
    assert_eq!(metadata.string_id, Some(42));
}

#[tokio::test]
async fn writes_reordered_high_level_entries_to_their_original_plugin_fields() {
    let plugin = build_plugin(
        localized_header(),
        vec![
            build_major(
                "WEAP",
                0x800,
                0,
                vec![build_subrecord("FULL", &1u32.to_le_bytes())],
            ),
            build_major(
                "ARMO",
                0x801,
                0,
                vec![build_subrecord("FULL", &2u32.to_le_bytes())],
            ),
        ],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    strings.insert(2, "Iron Armor");
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        write_strings_file(
            "Data/Strings/MyMod_English.STRINGS",
            &strings,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
    ]);
    let mut localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    localization.entries_mut().swap(0, 1);
    let armor = localization
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.text() == "Iron Armor")
        .expect("armor entry should exist");
    armor.set_text("Steel Armor");

    let output = write_localization(
        localization,
        WriteOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();
    let parsed_strings = stringer_plugin::parse_strings_file(
        output
            .get("Data/Strings/MyMod_English.STRINGS")
            .expect("strings output"),
        GameRelease::SkyrimSe,
        Language::English,
    )
    .unwrap();

    assert_eq!(parsed_strings.get(1), Some("Iron Sword"));
    assert_eq!(parsed_strings.get(2), Some("Steel Armor"));
}

#[tokio::test]
async fn matches_strings_file_mod_names_case_insensitively() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        write_strings_file(
            "Data/Strings/mymod_English.STRINGS",
            &strings,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
    ]);

    let localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    assert_eq!(localization.entries()[0].text(), "Iron Sword");
}

#[tokio::test]
async fn rejects_ambiguous_multiple_plugin_files() {
    let first = build_plugin(localized_header(), vec![]);
    let second = build_plugin(localized_header(), vec![]);
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/First.esp", Bytes::from(first)),
        FileAsset::new("Data/Second.esp", Bytes::from(second)),
    ]);

    let error = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, PluginError::AmbiguousPluginFiles { .. }));
}

#[tokio::test]
async fn rejects_duplicate_matching_strings_files() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let first = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &strings,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    let second = FileAsset::new(
        "Data/Strings/Sub/MyMod_English.STRINGS",
        first.bytes().clone(),
    );
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        first,
        second,
    ]);

    let error = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, PluginError::DuplicateStringsFile { .. }));
}

#[tokio::test]
async fn reports_missing_localized_string_ids() {
    let plugin = build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &99u32.to_le_bytes())],
        )],
    );
    let strings = StringsFile::new(StringsKind::Normal, Language::English);
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/MyMod.esp", Bytes::from(plugin)),
        write_strings_file(
            "Data/Strings/MyMod_English.STRINGS",
            &strings,
            GameRelease::SkyrimSe,
        )
        .unwrap(),
    ]);

    let error = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, PluginError::MissingStringId { id: 99, .. }));
}

#[tokio::test]
async fn edits_non_localized_embedded_strings_through_high_level_api() {
    let plugin = build_plugin_with_flags(
        0,
        Vec::new(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", b"Iron Sword\0")],
        )],
    );
    let bundle = FileBundle::new(vec![FileAsset::new("Data/MyMod.esp", Bytes::from(plugin))]);

    let mut localization = read_localization(
        bundle,
        ReadOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();
    localization.entries_mut()[0].set_text("Steel Sword");
    let output = write_localization(
        localization,
        WriteOptions::new(GameRelease::SkyrimSe, Language::English),
    )
    .await
    .unwrap();

    assert!(
        output
            .get("Data/MyMod.esp")
            .unwrap()
            .bytes()
            .windows(b"Steel Sword\0".len())
            .any(|w| w == b"Steel Sword\0")
    );
    assert_eq!(output.strings().count(), 0);
}
