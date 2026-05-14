use std::{collections::BTreeMap, fs};

use camino::Utf8Path;
use serde_json::json;

use crate::{
    AdaptCatalog, AdaptError, AdaptImportOptions, AdaptQuality, ParsedEntry, binary::BinaryReader,
    catalog_from_entries, insert_non_empty, malformed,
};

pub(crate) fn read(
    path: &Utf8Path,
    options: &AdaptImportOptions,
) -> Result<AdaptCatalog, AdaptError> {
    let bytes = fs::read(path).map_err(|source| AdaptError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let mut reader = BinaryReader::new(&bytes);
    let header = reader
        .read_u32()
        .map_err(|message| malformed(path, "XT SST", message))?;
    let version = version(header).ok_or_else(|| {
        malformed(
            path,
            "XT SST",
            format!("unsupported SST header {header:#010X}"),
        )
    })?;
    if version > 3 {
        let _flag = reader
            .read_u8()
            .map_err(|message| malformed(path, "XT SST", message))?;
    }
    let masters = read_masters(path, &mut reader, version)?;
    let colab_labels = read_colab_labels(path, &mut reader, version)?;
    let mut parsed = Vec::new();
    let mut row_index = 0usize;
    while !reader.is_empty() {
        parsed.push(read_row(
            path,
            &mut reader,
            version,
            row_index,
            &masters,
            &colab_labels,
        )?);
        row_index += 1;
    }
    Ok(catalog_from_entries(
        "xt_sst",
        parsed,
        options,
        options.game.clone(),
    ))
}

fn read_masters(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
) -> Result<Vec<String>, AdaptError> {
    let mut masters = Vec::new();
    if version >= 8 {
        let count = reader
            .read_i32()
            .map_err(|message| malformed(path, "XT SST", message))?;
        for _ in 0..count {
            masters.push(
                reader
                    .read_utf16_i32_string()
                    .map_err(|message| malformed(path, "XT SST", message))?,
            );
        }
    }
    Ok(masters)
}

fn read_colab_labels(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
) -> Result<BTreeMap<String, String>, AdaptError> {
    let mut labels = BTreeMap::<String, String>::new();
    if version >= 7 {
        let count = reader
            .read_i32()
            .map_err(|message| malformed(path, "XT SST", message))?;
        for _ in 0..count {
            let id = reader
                .read_i32()
                .map_err(|message| malformed(path, "XT SST", message))?;
            let label = reader
                .read_utf16_i32_string()
                .map_err(|message| malformed(path, "XT SST", message))?;
            labels.insert(id.to_string(), label);
        }
    }
    Ok(labels)
}

fn read_row(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
    row_index: usize,
    masters: &[String],
    colab_labels: &BTreeMap<String, String>,
) -> Result<ParsedEntry, AdaptError> {
    let list_index = reader
        .read_u8()
        .map_err(|message| malformed(path, "XT SST", message))?;
    let pointer = read_pointer(path, reader, version)?;
    let colab_id = if version > 5 {
        Some(
            reader
                .read_u8()
                .map_err(|message| malformed(path, "XT SST", message))?,
        )
    } else {
        None
    };
    let params = reader
        .read_u8()
        .map_err(|message| malformed(path, "XT SST", message))?;
    let source = reader
        .read_utf16_i32_string()
        .map_err(|message| malformed(path, "XT SST", message))?;
    let target = reader
        .read_utf16_i32_string()
        .map_err(|message| malformed(path, "XT SST", message))?;
    let mut context = pointer.context;
    context.insert(
        "strings_kind".to_string(),
        strings_kind(list_index).to_string(),
    );
    let colab_label = colab_id.and_then(|id| colab_labels.get(&id.to_string()).cloned());
    Ok(ParsedEntry {
        source,
        target,
        context,
        origin: json!({
            "format": "xt_sst",
            "version": version,
            "row": row_index,
            "params": params,
            "colab_id": colab_id,
            "colab_label": colab_label,
            "masters": masters,
        }),
        quality: quality(params),
    })
}

fn read_pointer(
    path: &Utf8Path,
    reader: &mut BinaryReader<'_>,
    version: u32,
) -> Result<PointerContext, AdaptError> {
    let mut context = BTreeMap::new();
    if version > 1 {
        let string_id = reader
            .read_i32()
            .map_err(|message| malformed(path, "XT SST", message))?;
        let form_id = reader
            .read_u32()
            .map_err(|message| malformed(path, "XT SST", message))?;
        if version > 4 {
            let record_type = reader
                .read_ascii(4)
                .map_err(|message| malformed(path, "XT SST", message))?;
            insert_non_empty(&mut context, "record_type", trim_sig(&record_type));
        }
        let subrecord = reader
            .read_ascii(4)
            .map_err(|message| malformed(path, "XT SST", message))?;
        insert_non_empty(&mut context, "subrecord", trim_sig(&subrecord));
        context.insert("form_id".to_string(), format!("{form_id:#010X}"));
        context.insert("string_id".to_string(), string_id.to_string());
        if version > 2 {
            let index = reader
                .read_u16()
                .map_err(|message| malformed(path, "XT SST", message))?;
            context.insert("field_index".to_string(), index.to_string());
        }
        if version > 3 {
            let index_max = reader
                .read_u16()
                .map_err(|message| malformed(path, "XT SST", message))?;
            let record_hash = reader
                .read_u32()
                .map_err(|message| malformed(path, "XT SST", message))?;
            context.insert("field_index_max".to_string(), index_max.to_string());
            context.insert("record_hash".to_string(), format!("{record_hash:#010X}"));
        }
    }
    Ok(PointerContext { context })
}

struct PointerContext {
    context: BTreeMap<String, String>,
}

fn quality(params: u8) -> AdaptQuality {
    let locked = params & 0b0000_0010 != 0;
    let validated = params & 0b0000_1000 != 0;
    if locked || validated {
        AdaptQuality::Confirmed
    } else {
        AdaptQuality::Imported
    }
}

fn version(header: u32) -> Option<u32> {
    match header {
        0x3255_5353 => Some(1),
        0x3355_5353 => Some(2),
        0x3455_5353 => Some(3),
        0x3555_5353 => Some(4),
        0x3655_5353 => Some(5),
        0x3755_5353 => Some(6),
        0x3855_5353 => Some(7),
        0x3955_5353 => Some(8),
        _ => None,
    }
}

fn strings_kind(index: u8) -> &'static str {
    match index {
        0 => "strings",
        1 => "dlstrings",
        2 => "ilstrings",
        _ => "unknown",
    }
}

fn trim_sig(value: &str) -> String {
    value.trim_matches('\0').trim().to_string()
}
