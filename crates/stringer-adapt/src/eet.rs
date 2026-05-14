use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use serde_json::{Value, json};
use stringer_core::binary::{BinaryReader, Endian};

use crate::{
    AdaptCatalog, AdaptError, AdaptImportOptions, AdaptQuality, ParsedEntry,
    binary_ext::AdaptBinaryReaderExt, catalog_from_entries, insert_non_empty, malformed,
    xml::xml_rows,
};

pub(crate) fn read_binary(
    path: &Utf8Path,
    options: &AdaptImportOptions,
) -> Result<AdaptCatalog, AdaptError> {
    let bytes = fs::read(path).map_err(|source| AdaptError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let mut reader = BinaryReader::new(&bytes, Endian::Little);
    let magic = reader
        .take(4, "EET header")
        .map_err(|message| malformed(path, "EET", message))?;
    if magic != b"EET_" {
        return Err(malformed(path, "EET", "missing EET_ header"));
    }
    let table_type = reader
        .read_i32("table type")
        .map_err(|message| malformed(path, "EET", message))?;
    let version = reader
        .read_u32("version")
        .map_err(|message| malformed(path, "EET", message))?;
    let mut game = options.game.clone();
    let mut parsed = Vec::new();
    while !reader.is_empty() {
        let tag = reader
            .read_ascii(4, "chunk tag")
            .map_err(|message| malformed(path, "EET", message))?;
        match tag.as_str() {
            "GAME" => {
                let value = reader
                    .read_utf8_u16_string("game name")
                    .map_err(|message| malformed(path, "EET", message))?;
                if game.is_none() && !value.is_empty() {
                    game = Some(value);
                }
            }
            "LINE" => {
                let count = reader
                    .read_u32("line count")
                    .map_err(|message| malformed(path, "EET", message))?;
                for row_index in 0..count {
                    parsed.push(read_binary_row(
                        path,
                        &mut reader,
                        version,
                        table_type,
                        row_index,
                    )?);
                }
            }
            "PHRA" => read_dictionary(path, &mut reader, version)?,
            _ => return Err(malformed(path, "EET", format!("unsupported chunk `{tag}`"))),
        }
    }
    Ok(catalog_from_entries("eet", parsed, options, game))
}

pub(crate) fn read_xml(
    path: &Utf8Path,
    options: &AdaptImportOptions,
) -> Result<AdaptCatalog, AdaptError> {
    let text = fs::read_to_string(path).map_err(|source| AdaptError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let parsed = xml_rows(path, &text)?
        .into_iter()
        .filter(|(_, row)| is_text_row(row))
        .enumerate()
        .map(|(index, (table, row))| parsed_map_row("eet_xml", Some(table), index, row))
        .collect::<Vec<_>>();
    Ok(catalog_from_entries(
        "eet_xml",
        parsed,
        options,
        options.game.clone(),
    ))
}

pub(crate) fn read_json(
    path: &Utf8Path,
    options: &AdaptImportOptions,
) -> Result<AdaptCatalog, AdaptError> {
    let text = fs::read_to_string(path).map_err(|source| AdaptError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let value = serde_json::from_str::<Value>(&text).map_err(|source| AdaptError::Json {
        path: path.to_owned(),
        source,
    })?;
    let rows = match value {
        Value::Array(rows) => rows,
        Value::Object(mut object) => object
            .remove("rows")
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_else(|| vec![Value::Object(object)]),
        _ => Vec::new(),
    };
    let parsed = rows
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| {
            let Value::Object(object) = value else {
                return None;
            };
            Some(parsed_json_row(index, object.into_iter().collect()))
        })
        .collect::<Vec<_>>();
    Ok(catalog_from_entries(
        "eet_json",
        parsed,
        options,
        options.game.clone(),
    ))
}

fn read_binary_row(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
    table_type: i32,
    row_index: u32,
) -> Result<ParsedEntry, AdaptError> {
    let mut payload_reader;
    let source = if version >= 2 {
        let size = reader
            .read_u32("row payload size")
            .map_err(|message| malformed(path, "EET", message))?;
        let payload = reader
            .take(size as usize, "row payload")
            .map_err(|message| malformed(path, "EET", message))?;
        payload_reader = BinaryReader::new(payload, Endian::Little);
        &mut payload_reader
    } else {
        reader
    };
    let grup = source
        .read_utf8_u32_string("record type")
        .map_err(|message| malformed(path, "EET", message))?;
    let id = source
        .read_utf8_u32_string("form id")
        .map_err(|message| malformed(path, "EET", message))?;
    let edid = source
        .read_utf8_u32_string("editor id")
        .map_err(|message| malformed(path, "EET", message))?;
    let champ = source
        .read_utf8_u32_string("subrecord")
        .map_err(|message| malformed(path, "EET", message))?;
    let original = source
        .read_utf8_u32_string("source text")
        .map_err(|message| malformed(path, "EET", message))?;
    let traduit = source
        .read_utf8_u32_string("target text")
        .map_err(|message| malformed(path, "EET", message))?;
    let perso = source
        .read_utf8_u32_string("personal text")
        .map_err(|message| malformed(path, "EET", message))?;
    let index = source
        .read_i32("field index")
        .map_err(|message| malformed(path, "EET", message))?;
    let status = source
        .read_i16("status")
        .map_err(|message| malformed(path, "EET", message))?;
    let text_id = source
        .read_i32("text id")
        .map_err(|message| malformed(path, "EET", message))?;
    let comment = source
        .read_utf8_u32_string("comment")
        .map_err(|message| malformed(path, "EET", message))?;
    let mut context = BTreeMap::new();
    insert_non_empty(&mut context, "record_type", grup);
    insert_form_id(&mut context, &id);
    insert_non_empty(&mut context, "edid", edid);
    insert_non_empty(&mut context, "subrecord", champ);
    context.insert("field_index".to_string(), index.to_string());
    if text_id != 0 {
        context.insert("text_id".to_string(), text_id.to_string());
    }
    Ok(ParsedEntry {
        source: original,
        target: traduit,
        context,
        origin: json!({
            "format": "eet",
            "version": version,
            "table_type": table_type,
            "row": row_index,
            "status": status,
            "personal": perso,
            "comment": comment,
        }),
        quality: quality(status),
    })
}

fn read_dictionary(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
) -> Result<(), AdaptError> {
    if version >= 2 {
        let _byte_len = reader
            .read_u32("dictionary byte length")
            .map_err(|message| malformed(path, "EET", message))?;
    }
    let count = reader
        .read_u32("dictionary entry count")
        .map_err(|message| malformed(path, "EET", message))?;
    for _ in 0..count {
        let _ = reader
            .read_utf8_u16_string("dictionary source")
            .map_err(|message| malformed(path, "EET", message))?;
        let _ = reader
            .read_utf8_u16_string("dictionary target")
            .map_err(|message| malformed(path, "EET", message))?;
    }
    Ok(())
}

fn parsed_json_row(index: usize, row: BTreeMap<String, Value>) -> ParsedEntry {
    let get = |keys: &[&str]| -> String {
        keys.iter()
            .find_map(|key| value_string(row.get(*key)))
            .unwrap_or_default()
    };
    let mut context = BTreeMap::new();
    let type_value = get(&["type", "TYPE"]);
    if let Some((record, field)) = type_value.split_once(' ') {
        insert_non_empty(&mut context, "record_type", record.trim().to_string());
        insert_non_empty(&mut context, "subrecord", field.trim().to_string());
    }
    let form = get(&["form_id", "ID"]);
    if let Some((id, file)) = form.split_once('|') {
        insert_form_id(&mut context, id);
        insert_non_empty(&mut context, "source_file", file.to_string());
    } else {
        insert_form_id(&mut context, &form);
    }
    insert_non_empty(&mut context, "edid", get(&["editor_id", "EDID"]));
    insert_non_empty(&mut context, "field_index", get(&["index", "INDEX"]));
    let status = get(&["status", "STATUS"]);
    ParsedEntry {
        source: get(&["original", "ORIGINAL"]),
        target: get(&["string", "TRADUIT", "translated"]),
        context,
        origin: json!({ "format": "eet_json", "row": index, "status": status }),
        quality: quality_text(&status),
    }
}

fn parsed_map_row(
    format: &'static str,
    table: Option<String>,
    index: usize,
    row: BTreeMap<String, String>,
) -> ParsedEntry {
    let get = |keys: &[&str]| -> String {
        keys.iter()
            .find_map(|key| row.get(*key).cloned())
            .unwrap_or_default()
    };
    let mut context = BTreeMap::new();
    insert_non_empty(&mut context, "record_type", get(&["GRUP", "group"]));
    insert_form_id(&mut context, &get(&["ID", "form_id"]));
    insert_non_empty(&mut context, "edid", get(&["EDID", "editor_id"]));
    insert_non_empty(&mut context, "subrecord", get(&["CHAMP", "field"]));
    insert_non_empty(&mut context, "field_index", get(&["INDEX", "index"]));
    let status = get(&["STATUS", "status"]);
    ParsedEntry {
        source: get(&["ORIGINAL", "original"]),
        target: get(&["TRADUIT", "string", "translated"]),
        context,
        origin: json!({ "format": format, "table": table, "row": index, "status": status }),
        quality: quality_text(&status),
    }
}

fn value_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn insert_form_id(context: &mut BTreeMap<String, String>, value: &str) {
    if let Some(form_id) = format_form_id(value) {
        context.insert("form_id".to_string(), form_id);
    }
}

fn format_form_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if trimmed.is_empty() {
        return None;
    }
    let parsed = if trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        u32::from_str_radix(trimmed, 16).ok()
    } else {
        trimmed.parse::<u32>().ok()
    }?;
    Some(format!("{parsed:#010X}"))
}

fn quality(status: i16) -> AdaptQuality {
    match status {
        99 => AdaptQuality::Confirmed,
        50 => AdaptQuality::Machine,
        -1 => AdaptQuality::Rejected,
        _ => AdaptQuality::Imported,
    }
}

fn quality_text(status: &str) -> AdaptQuality {
    if status.trim() == "-1" {
        return AdaptQuality::Rejected;
    }
    let normalized = status
        .chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' '))
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match normalized.as_str() {
        "99" | "translationcomplete" | "notranslationrequired" => AdaptQuality::Confirmed,
        "50" => AdaptQuality::Machine,
        _ => AdaptQuality::Imported,
    }
}

fn is_text_row(row: &BTreeMap<String, String>) -> bool {
    row.contains_key("ORIGINAL")
        || row.contains_key("original")
        || row.contains_key("TRADUIT")
        || row.contains_key("string")
        || row.contains_key("translated")
}
