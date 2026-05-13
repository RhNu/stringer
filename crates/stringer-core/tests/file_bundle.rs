use bytes::Bytes;
use stringer_core::{
    Diagnostic, FileAsset, FileBundle, FileFormat, FileRole, PluginStringMetadata,
    PluginStringStorage, SourceSpan, StringEntry, StringEntryContext, StringEntrySource,
    StringerCoreError,
};

#[test]
fn identifies_plugin_and_strings_files_from_logical_paths() {
    let plugin = FileAsset::new("Data/My Mod.esp", Bytes::from_static(b"plugin"));
    let strings = FileAsset::new(
        "Data/Strings/My Mod_English.DLSTRINGS",
        Bytes::from_static(b"strings"),
    );
    let pex = FileAsset::new("Data/Scripts/MyScript.pex", Bytes::from_static(b"pex"));
    let unknown = FileAsset::new("readme.txt", Bytes::from_static(b"readme"));

    assert_eq!(plugin.role(), FileRole::Plugin);
    assert_eq!(plugin.format(), FileFormat::Esp);
    assert_eq!(strings.role(), FileRole::Strings);
    assert_eq!(strings.format(), FileFormat::DlStrings);
    assert_eq!(pex.role(), FileRole::Pex);
    assert_eq!(pex.format(), FileFormat::Pex);
    assert_eq!(unknown.role(), FileRole::Unknown);
    assert_eq!(unknown.format(), FileFormat::Unknown);
}

#[test]
fn normalizes_bundle_lookup_paths_case_insensitively() {
    let bundle = FileBundle::new(vec![
        FileAsset::new(
            "Data/Strings/My Mod_English.STRINGS",
            Bytes::from_static(b"abc"),
        ),
        FileAsset::with_role(
            "Data/My Mod.esp",
            Bytes::from_static(b"plugin"),
            FileRole::Plugin,
        ),
    ]);

    assert!(bundle.get("data/strings/my mod_english.strings").is_some());
    assert!(
        bundle
            .plugins()
            .any(|asset| asset.path().as_str() == "Data/My Mod.esp")
    );
    assert_eq!(bundle.strings().count(), 1);
}

#[test]
fn filters_pex_files_from_bundle() {
    let bundle = FileBundle::new(vec![
        FileAsset::new("Data/Scripts/QuestScript.pex", Bytes::from_static(b"one")),
        FileAsset::new("Data/Scripts/Helper.PEX", Bytes::from_static(b"two")),
        FileAsset::new("Data/My Mod.esp", Bytes::from_static(b"plugin")),
    ]);

    assert_eq!(bundle.pex().count(), 2);
}

#[test]
fn rejects_duplicate_logical_paths_in_bundle() {
    let error = FileBundle::try_new(vec![
        FileAsset::new("Data/A.esp", Bytes::from_static(b"one")),
        FileAsset::new("data/a.ESP", Bytes::from_static(b"two")),
    ])
    .unwrap_err();

    assert!(matches!(error, StringerCoreError::DuplicatePath { .. }));
}

#[test]
fn diagnostics_preserve_message_severity_and_source_span() {
    let diagnostic = Diagnostic::error(
        "strings directory points outside the file",
        Some(SourceSpan::new("Data/Strings/A.STRINGS", 12, 8)),
    );

    assert_eq!(
        diagnostic.message(),
        "strings directory points outside the file"
    );
    assert!(diagnostic.is_error());
    assert_eq!(diagnostic.span().unwrap().offset(), 12);
    assert_eq!(diagnostic.span().unwrap().len(), 8);
}

#[test]
fn string_entries_track_text_changes_and_source_metadata() {
    let source = StringEntrySource::Plugin(PluginStringMetadata {
        path: "Data/My Mod.esp".into(),
        record_type: "WEAP".to_string(),
        form_id: 0x800,
        subrecord: "FULL".to_string(),
        strings_kind: "STRINGS".to_string(),
        field_source: "Normal".to_string(),
        storage: PluginStringStorage::Localized,
        string_id: Some(42),
    });
    let mut entry = StringEntry::new(
        "plugin:Data/My Mod.esp:WEAP:00000800:FULL:42",
        "Iron Sword",
        source,
        StringEntryContext::default(),
    );

    assert_eq!(entry.id(), "plugin:Data/My Mod.esp:WEAP:00000800:FULL:42");
    assert_eq!(entry.text(), "Iron Sword");
    assert!(!entry.is_dirty());

    entry.set_text("Steel Sword");

    assert_eq!(entry.text(), "Steel Sword");
    assert!(entry.is_dirty());
    let StringEntrySource::Plugin(metadata) = entry.source() else {
        panic!("expected plugin metadata");
    };
    assert_eq!(metadata.record_type, "WEAP");
    assert_eq!(metadata.form_id, 0x800);
    assert_eq!(metadata.string_id, Some(42));
}
