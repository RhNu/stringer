use std::fs;

use camino::Utf8PathBuf;
use serde_json::Value;
use stringer_adapt::{
    AdaptFormat, AdaptImportOptions, AdaptQuality, merge_memory_jsonl, read_adapt_catalog,
    write_memory_jsonl,
};
use stringer_pipeline::KnowledgeLayer;

#[test]
fn eet_binary_v3_maps_line_context_origin_and_quality() {
    let input = test_path("eet-v3.eet");
    fs::write(&input, eet_v3_fixture()).unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetBinary)).unwrap();

    assert_eq!(catalog.summary.total_entries, 1);
    assert_eq!(catalog.summary.written_entries, 1);
    assert!(catalog.diagnostics.is_empty());
    let entry = &catalog.entries[0];
    assert_eq!(entry.source, "Iron Sword");
    assert_eq!(entry.target, "铁剑");
    assert_eq!(entry.quality, AdaptQuality::Confirmed);
    assert!(!entry.context.contains_key("game"));
    assert_eq!(entry.context["record_type"], "WEAP");
    assert_eq!(entry.context["subrecord"], "FULL");
    assert_eq!(entry.context["form_id"], "0x00001234");
    assert_eq!(entry.context["edid"], "IronSword");
    assert_eq!(entry.context["field_index"], "7");
    assert_eq!(entry.origin["format"], "eet");
    assert_eq!(entry.origin["version"], 3);
    assert_eq!(entry.origin["status"], 99);
}

#[test]
fn eet_xml_maps_database_rows_to_memory_entries() {
    let input = test_path("eet.xml");
    fs::write(
        &input,
        r#"<NewDataSet>
  <BDD>
    <GRUP>WEAP</GRUP>
    <ID>00001234</ID>
    <EDID>IronSword</EDID>
    <CHAMP>FULL</CHAMP>
    <ORIGINAL>Iron Sword</ORIGINAL>
    <TRADUIT>铁剑</TRADUIT>
    <PERSO />
    <INDEX>3</INDEX>
  </BDD>
</NewDataSet>"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetXml)).unwrap();

    assert_eq!(catalog.entries.len(), 1);
    let entry = &catalog.entries[0];
    assert_eq!(entry.source, "Iron Sword");
    assert_eq!(entry.target, "铁剑");
    assert_eq!(entry.context["record_type"], "WEAP");
    assert_eq!(entry.context["subrecord"], "FULL");
    assert_eq!(entry.context["field_index"], "3");
    assert_eq!(entry.origin["format"], "eet_xml");
}

#[test]
fn eet_xml_decodes_numeric_entities_and_rejects_malformed_xml() {
    let input = test_path("eet-entities.xml");
    fs::write(
        &input,
        r#"<?xml version="1.0"?>
<NewDataSet>
  <BDD>
    <ORIGINAL>Iron &#x26; Steel</ORIGINAL>
    <TRADUIT><![CDATA[铁&#21073;]]></TRADUIT>
  </BDD>
</NewDataSet>"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetXml)).unwrap();

    assert_eq!(catalog.entries[0].source, "Iron & Steel");
    assert_eq!(catalog.entries[0].target, "铁&#21073;");

    let malformed = test_path("eet-malformed.xml");
    fs::write(&malformed, "<NewDataSet><BDD><ORIGINAL>Iron").unwrap();

    assert!(read_adapt_catalog(&malformed, options(AdaptFormat::EetXml)).is_err());
}

#[test]
fn eet_xml_ignores_inline_schema_rows() {
    let input = test_path("eet-schema.xml");
    fs::write(
        &input,
        r#"<NewDataSet>
  <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
    <xs:element name="NewDataSet" />
  </xs:schema>
  <BDD>
    <ORIGINAL>Iron Sword</ORIGINAL>
    <TRADUIT>铁剑</TRADUIT>
  </BDD>
</NewDataSet>"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetXml)).unwrap();

    assert_eq!(catalog.summary.total_entries, 1);
    assert_eq!(catalog.entries.len(), 1);
    assert!(catalog.diagnostics.is_empty());
}

#[test]
fn eet_json_maps_dds_rows_to_memory_entries() {
    let input = test_path("eet.json");
    fs::write(
        &input,
        r#"[{
  "editor_id": "IronSword",
  "form_id": "00001234|Skyrim.esm",
  "index": 5,
  "type": "WEAP FULL",
  "original": "Iron Sword",
  "string": "铁剑",
  "status": 99
}]"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetJson)).unwrap();

    assert_eq!(catalog.entries.len(), 1);
    let entry = &catalog.entries[0];
    assert_eq!(entry.context["record_type"], "WEAP");
    assert_eq!(entry.context["subrecord"], "FULL");
    assert_eq!(entry.context["form_id"], "0x00001234");
    assert_eq!(entry.context["source_file"], "Skyrim.esm");
    assert_eq!(entry.origin["format"], "eet_json");
}

#[test]
fn eet_json_translation_complete_status_maps_to_confirmed_quality() {
    let input = test_path("eet-status.json");
    fs::write(
        &input,
        r#"[{
  "original": "Iron Sword",
  "string": "铁剑",
  "status": "TranslationComplete"
}]"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetJson)).unwrap();

    assert_eq!(catalog.entries[0].quality, AdaptQuality::Confirmed);
    assert_eq!(catalog.entries[0].origin["status"], "TranslationComplete");
}

#[test]
fn eet_json_negative_status_maps_to_rejected_quality() {
    let input = test_path("eet-rejected.json");
    fs::write(
        &input,
        r#"[{
  "original": "Iron Sword",
  "string": "铁剑",
  "status": -1
}]"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetJson)).unwrap();

    assert_eq!(catalog.entries[0].quality, AdaptQuality::Rejected);
}

#[test]
fn xt_sst_v8_maps_record_context_status_and_master_metadata() {
    let input = test_path("current.sst");
    fs::write(&input, xt_sst_fixture(0x3955_5353, true, true)).unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::XtSst)).unwrap();

    assert_eq!(catalog.entries.len(), 1);
    let entry = &catalog.entries[0];
    assert_eq!(entry.source, "Iron Sword");
    assert_eq!(entry.target, "铁剑");
    assert_eq!(entry.quality, AdaptQuality::Confirmed);
    assert_eq!(entry.context["record_type"], "WEAP");
    assert_eq!(entry.context["subrecord"], "FULL");
    assert_eq!(entry.context["form_id"], "0x00001234");
    assert_eq!(entry.context["string_id"], "123");
    assert_eq!(entry.context["strings_kind"], "strings");
    assert_eq!(entry.context["field_index"], "7");
    assert_eq!(entry.origin["format"], "xt_sst");
    assert_eq!(entry.origin["version"], 8);
    assert_eq!(entry.origin["masters"][0], "Skyrim.esm");
}

#[test]
fn xt_sst_historical_header_without_colab_is_supported() {
    let input = test_path("old.sst");
    fs::write(&input, xt_sst_fixture(0x3655_5353, false, false)).unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::XtSst)).unwrap();

    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].origin["version"], 5);
    assert_eq!(catalog.entries[0].context["record_type"], "WEAP");
}

#[test]
fn xt_sst_v3_without_placeholder_uses_historical_pointer_layout() {
    let input = test_path("v3.sst");
    fs::write(&input, xt_sst_v3_fixture()).unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::XtSst)).unwrap();

    assert_eq!(catalog.entries.len(), 1);
    let entry = &catalog.entries[0];
    assert_eq!(entry.source, "Iron Sword");
    assert_eq!(entry.target, "铁剑");
    assert_eq!(entry.context["subrecord"], "FULL");
    assert_eq!(entry.context["form_id"], "0x00001234");
    assert_eq!(entry.context["field_index"], "7");
    assert_eq!(entry.origin["version"], 3);
}

#[test]
fn write_memory_jsonl_emits_stringer_memory_entries() {
    let input = test_path("eet-v3-write.eet");
    let output = test_path("memory.jsonl");
    fs::write(&input, eet_v3_fixture()).unwrap();
    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetBinary)).unwrap();

    let summary = write_memory_jsonl(&catalog, &output).unwrap();

    assert_eq!(summary.written_entries, 1);
    let line = fs::read_to_string(output).unwrap();
    let row: Value = serde_json::from_str(line.trim()).unwrap();
    assert!(row["id"].as_str().unwrap().starts_with("adapt:eet:"));
    assert_eq!(row["source"], "Iron Sword");
    assert_eq!(row["target"], "铁剑");
    assert_eq!(row["source_locale"], "en");
    assert_eq!(row["target_locale"], "zh-Hans");
    assert_eq!(row["quality"], "confirmed");
    assert_eq!(row["context"]["record_type"], "WEAP");
}

#[test]
fn write_memory_jsonl_suffixes_duplicate_ids_so_memory_can_load() {
    let input = test_path("duplicates.json");
    let output = test_path("duplicates-memory.jsonl");
    fs::write(
        &input,
        r#"[
  {"original":"Shared","string":"共享"},
  {"original":"Shared","string":"共享"}
]"#,
    )
    .unwrap();
    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetJson)).unwrap();

    write_memory_jsonl(&catalog, &output).unwrap();

    let text = fs::read_to_string(output).unwrap();
    let ids = text
        .lines()
        .map(|line| {
            serde_json::from_str::<Value>(line).unwrap()["id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_ne!(ids[0], ids[1]);
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_memory_jsonl("knowledge/memory/adapt.jsonl", &text)
        .unwrap();
}

#[test]
fn merge_memory_jsonl_appends_new_entries_without_rewriting_existing_rows() {
    let input = test_path("merge-new.eet");
    let output = test_path("merge-new-memory.jsonl");
    fs::write(&input, eet_v3_fixture()).unwrap();
    fs::write(
        &output,
        r#"{"id":"manual:1","source":"Steel Sword","target":"钢剑","source_locale":"en","target_locale":"zh-Hans","quality":"confirmed"}"#,
    )
    .unwrap();
    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetBinary)).unwrap();

    merge_memory_jsonl(&catalog, &output).unwrap();

    let text = fs::read_to_string(output).unwrap();
    let rows = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "manual:1");
    assert_eq!(rows[1]["source"], "Iron Sword");
}

#[test]
fn merge_memory_jsonl_is_idempotent_for_same_catalog() {
    let input = test_path("merge-idempotent.eet");
    let output = test_path("merge-idempotent-memory.jsonl");
    fs::write(&input, eet_v3_fixture()).unwrap();
    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetBinary)).unwrap();

    merge_memory_jsonl(&catalog, &output).unwrap();
    merge_memory_jsonl(&catalog, &output).unwrap();

    let text = fs::read_to_string(output).unwrap();
    assert_eq!(text.lines().count(), 1);
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_memory_jsonl("knowledge/memory/adapt/merge-idempotent.eet.jsonl", &text)
        .unwrap();
}

#[test]
fn merge_memory_jsonl_replaces_existing_row_with_same_id() {
    let input = test_path("merge-replace.eet");
    let output = test_path("merge-replace-memory.jsonl");
    fs::write(&input, eet_v3_fixture()).unwrap();
    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetBinary)).unwrap();
    let entry = &catalog.entries[0];
    fs::write(
        &output,
        format!(
            r#"{{"id":"{}","source":"{}","target":"{}","source_locale":"en","target_locale":"zh-Hans","context":{},"quality":"machine"}}"#,
            entry.id,
            entry.source,
            entry.target,
            serde_json::to_string(&entry.context).unwrap()
        ),
    )
    .unwrap();

    merge_memory_jsonl(&catalog, &output).unwrap();

    let text = fs::read_to_string(output).unwrap();
    let rows = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], entry.id);
    assert_eq!(rows[0]["quality"], "confirmed");
    assert_eq!(rows[0]["origin"]["format"], "eet");
}

#[test]
fn empty_source_or_target_rows_are_diagnosed_and_skipped() {
    let input = test_path("empty.json");
    fs::write(
        &input,
        r#"[{"original":"","string":"铁剑"},{"original":"Iron Sword","string":""}]"#,
    )
    .unwrap();

    let catalog = read_adapt_catalog(&input, options(AdaptFormat::EetJson)).unwrap();

    assert!(catalog.entries.is_empty());
    assert_eq!(catalog.summary.total_entries, 2);
    assert_eq!(catalog.summary.skipped_entries, 2);
    assert_eq!(catalog.diagnostics.len(), 2);
}

fn options(format: AdaptFormat) -> AdaptImportOptions {
    AdaptImportOptions {
        source_locale: "en".to_string(),
        target_locale: "zh-Hans".to_string(),
        game: None,
        format,
    }
}

fn test_path(name: &str) -> Utf8PathBuf {
    let path = std::env::temp_dir().join(format!("stringer-adapt-{}-{name}", std::process::id()));
    Utf8PathBuf::from_path_buf(path).unwrap()
}

fn eet_v3_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"EET_");
    push_i32(&mut bytes, 1);
    push_u32(&mut bytes, 3);
    bytes.extend_from_slice(b"GAME");
    push_u16_string(&mut bytes, "Skyrim");
    bytes.extend_from_slice(b"LINE");
    push_u32(&mut bytes, 1);
    let mut row = Vec::new();
    push_u32_string(&mut row, "WEAP");
    push_u32_string(&mut row, "00001234");
    push_u32_string(&mut row, "IronSword");
    push_u32_string(&mut row, "FULL");
    push_u32_string(&mut row, "Iron Sword");
    push_u32_string(&mut row, "铁剑");
    push_u32_string(&mut row, "");
    push_i32(&mut row, 7);
    row.extend_from_slice(&99i16.to_le_bytes());
    push_i32(&mut row, 42);
    push_u32_string(&mut row, "checked");
    row.extend_from_slice(&[1, 2, 3, 4]);
    push_u32(&mut bytes, row.len() as u32);
    bytes.extend_from_slice(&row);
    bytes
}

fn xt_sst_fixture(header: u32, has_colab_headers: bool, has_colab_id: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, header);
    bytes.push(0);
    if has_colab_headers {
        push_i32(&mut bytes, 1);
        push_i32_utf16(&mut bytes, "Skyrim.esm");
        push_i32(&mut bytes, 0);
    }
    bytes.push(0);
    push_i32(&mut bytes, 123);
    push_u32(&mut bytes, 0x1234);
    bytes.extend_from_slice(b"WEAP");
    bytes.extend_from_slice(b"FULL");
    bytes.extend_from_slice(&7u16.to_le_bytes());
    bytes.extend_from_slice(&8u16.to_le_bytes());
    push_u32(&mut bytes, 0xCAFE_BABE);
    if has_colab_id {
        bytes.push(0);
    }
    bytes.push(0b0000_1001);
    push_i32_utf16(&mut bytes, "Iron Sword");
    push_i32_utf16(&mut bytes, "铁剑");
    bytes
}

fn xt_sst_v3_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 0x3455_5353);
    bytes.push(0);
    push_i32(&mut bytes, 123);
    push_u32(&mut bytes, 0x1234);
    bytes.extend_from_slice(b"FULL");
    bytes.extend_from_slice(&7u16.to_le_bytes());
    bytes.push(0b0000_0001);
    push_i32_utf16(&mut bytes, "Iron Sword");
    push_i32_utf16(&mut bytes, "铁剑");
    bytes
}

fn push_u16_string(bytes: &mut Vec<u8>, value: &str) {
    let data = value.as_bytes();
    bytes.extend_from_slice(&(data.len() as u16).to_le_bytes());
    bytes.extend_from_slice(data);
}

fn push_u32_string(bytes: &mut Vec<u8>, value: &str) {
    let data = value.as_bytes();
    push_u32(bytes, data.len() as u32);
    bytes.extend_from_slice(data);
}

fn push_i32_utf16(bytes: &mut Vec<u8>, value: &str) {
    let data = value
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    push_i32(bytes, data.len() as i32);
    bytes.extend_from_slice(&data);
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
