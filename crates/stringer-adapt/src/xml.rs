use std::collections::BTreeMap;

use camino::Utf8Path;
use quick_xml::{
    Reader,
    events::{BytesRef, BytesText, Event},
    name::QName,
};

use crate::{AdaptError, malformed};

type XmlRow = (String, BTreeMap<String, String>);

pub(crate) fn xml_rows(path: &Utf8Path, text: &str) -> Result<Vec<XmlRow>, AdaptError> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    let mut rows = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut current_row = None::<(String, BTreeMap<String, String>)>;
    let mut current_field = None::<(String, String)>;
    loop {
        match reader
            .read_event()
            .map_err(|message| malformed(path, "EET XML", message.to_string()))?
        {
            Event::Start(event) => {
                let name = xml_name(event.name());
                if current_row.is_none() && stack.len() == 1 {
                    current_row = Some((name.clone(), BTreeMap::new()));
                } else if current_row.is_some() && stack.len() == 2 {
                    current_field = Some((name.clone(), String::new()));
                }
                stack.push(name);
            }
            Event::Empty(event) => {
                let name = xml_name(event.name());
                if let Some((_, fields)) = current_row.as_mut()
                    && stack.len() == 2
                {
                    fields.entry(name).or_default();
                }
            }
            Event::Text(event) => {
                if let Some((_, value)) = current_field.as_mut() {
                    value.push_str(&xml_text(path, &event)?);
                }
            }
            Event::CData(event) => {
                if let Some((_, value)) = current_field.as_mut() {
                    let decoded = event
                        .decode()
                        .map_err(|message| malformed(path, "EET XML", message.to_string()))?;
                    value.push_str(&decoded);
                }
            }
            Event::GeneralRef(event) => {
                if let Some((_, value)) = current_field.as_mut() {
                    value.push_str(&xml_reference(path, &event)?);
                }
            }
            Event::End(event) => {
                let name = xml_name(event.name());
                close_field(&mut current_row, &mut current_field, &name);
                close_row(path, &mut rows, &mut current_row, &mut stack, name)?;
            }
            Event::Eof => {
                if stack.is_empty() {
                    return Ok(rows);
                }
                return Err(malformed(path, "EET XML", "unexpected end of XML input"));
            }
            _ => {}
        }
    }
}

fn close_field(
    current_row: &mut Option<XmlRow>,
    current_field: &mut Option<(String, String)>,
    name: &str,
) {
    let Some((field, value)) = current_field.take() else {
        return;
    };
    if field == name {
        if let Some((_, fields)) = current_row.as_mut()
            && !value.is_empty()
        {
            fields.insert(field, value);
        }
    } else {
        *current_field = Some((field, value));
    }
}

fn close_row(
    path: &Utf8Path,
    rows: &mut Vec<XmlRow>,
    current_row: &mut Option<XmlRow>,
    stack: &mut Vec<String>,
    name: String,
) -> Result<(), AdaptError> {
    if let Some((row_name, fields)) = current_row.take() {
        if row_name == name {
            rows.push((row_name, fields));
        } else {
            *current_row = Some((row_name, fields));
        }
    }
    let open = stack
        .pop()
        .ok_or_else(|| malformed(path, "EET XML", "unexpected closing XML tag"))?;
    if open != name {
        return Err(malformed(
            path,
            "EET XML",
            format!("mismatched XML tags `{open}` and `{name}`"),
        ));
    }
    Ok(())
}

fn xml_name(name: QName<'_>) -> String {
    String::from_utf8_lossy(name.as_ref()).to_string()
}

fn xml_text(path: &Utf8Path, text: &BytesText<'_>) -> Result<String, AdaptError> {
    let decoded = text
        .decode()
        .map_err(|message| malformed(path, "EET XML", message.to_string()))?;
    quick_xml::escape::unescape(&decoded)
        .map(|value| value.into_owned())
        .map_err(|message| malformed(path, "EET XML", message.to_string()))
}

fn xml_reference(path: &Utf8Path, event: &BytesRef<'_>) -> Result<String, AdaptError> {
    let reference = event
        .decode()
        .map_err(|message| malformed(path, "EET XML", message.to_string()))?;
    if let Some(value) = reference.strip_prefix("#x") {
        let code = u32::from_str_radix(value, 16)
            .map_err(|message| malformed(path, "EET XML", message.to_string()))?;
        return char::from_u32(code)
            .map(|value| value.to_string())
            .ok_or_else(|| malformed(path, "EET XML", "invalid hexadecimal character reference"));
    }
    if let Some(value) = reference.strip_prefix('#') {
        let code = value
            .parse::<u32>()
            .map_err(|message| malformed(path, "EET XML", message.to_string()))?;
        return char::from_u32(code)
            .map(|value| value.to_string())
            .ok_or_else(|| malformed(path, "EET XML", "invalid decimal character reference"));
    }
    match reference.as_ref() {
        "amp" => Ok("&".to_string()),
        "lt" => Ok("<".to_string()),
        "gt" => Ok(">".to_string()),
        "quot" => Ok("\"".to_string()),
        "apos" => Ok("'".to_string()),
        _ => Err(malformed(
            path,
            "EET XML",
            format!("unrecognized entity reference `{reference}`"),
        )),
    }
}
