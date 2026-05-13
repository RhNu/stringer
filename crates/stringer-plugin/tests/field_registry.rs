use stringer_plugin::{LocalizedFieldSource, skyrim_localized_fields};

#[test]
fn registry_contains_all_known_skyrim_translated_field_declarations() {
    let fields = skyrim_localized_fields();

    assert_eq!(fields.len(), 81);
    assert!(
        fields
            .iter()
            .any(|f| f.major_record == "WEAP" && f.subrecord == "FULL")
    );
    assert!(
        fields
            .iter()
            .any(|f| f.major_record == "BOOK" && f.subrecord == "CNAM")
    );
    assert!(fields.iter().any(|f| f.major_record == "INFO"
        && f.subrecord == "NAM1"
        && f.source == LocalizedFieldSource::Il));
    assert!(
        fields
            .iter()
            .any(|f| f.major_record == "BPTD" && f.subrecord == "BPTN")
    );
    assert!(
        fields
            .iter()
            .any(|f| f.major_record == "WOOP" && f.subrecord == "TNAM")
    );
}
