use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use camino::Utf8PathBuf;
use serde_json::Value;
use stringer_core::{FileAsset, Language};
use stringer_pex::{
    PexFile, PexFunction, PexHeader, PexInstruction, PexLocal, PexObject, PexOpcode, PexState,
    PexValue,
};
use stringer_plugin::GameRelease;
use stringer_workspace::WorkspaceSettings;

pub fn settings() -> WorkspaceSettings {
    WorkspaceSettings {
        game_release: GameRelease::SkyrimSe,
        asset_language: Language::English,
        source_locale: "en".to_string(),
        target_locale: "zh-Hans".to_string(),
        global_knowledge_root: None,
    }
}

pub fn jsonl_rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

pub fn json_file(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

pub fn entry_rows(package: &Path, kind: &str, group: Option<&str>) -> Vec<Value> {
    jsonl_rows(&entry_file_path(package, kind, group))
}

pub fn write_entry_rows(package: &Path, kind: &str, rows: &str) {
    write_text(&entry_file_path(package, kind, None), rows);
}

fn entry_file_path(package: &Path, kind: &str, group: Option<&str>) -> PathBuf {
    let workspace = json_file(&package.join("workspace.json"));
    let file = workspace["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| file["kind"] == kind && group.is_none_or(|expected| file["group"] == expected))
        .expect("entry file");
    package.join(file["path"].as_str().unwrap())
}

pub fn write_text(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn write_bytes(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

pub fn utf8(path: &Path) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(path.to_path_buf()).unwrap()
}

pub fn decode_utf16le_bom(bytes: &[u8]) -> String {
    assert!(bytes.starts_with(&[0xFF, 0xFE]));
    let units = bytes[2..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).unwrap()
}

pub struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    pub fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "stringer_workspace_{label}_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}

pub fn build_localized_plugin() -> Vec<u8> {
    build_plugin(
        localized_header(),
        vec![build_major(
            "WEAP",
            0x800,
            0,
            vec![build_subrecord("FULL", &1u32.to_le_bytes())],
        )],
    )
}

fn localized_header() -> Vec<u8> {
    vec![0x80, 0, 0, 0]
}

fn build_plugin(tes4_content: Vec<u8>, top_level: Vec<Vec<u8>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(build_major(
        "TES4",
        0,
        0x80,
        vec![build_subrecord("HEDR", &tes4_content)],
    ));
    for item in top_level {
        bytes.extend(item);
    }
    bytes
}

fn build_major(record_type: &str, form_id: u32, flags: u32, subrecords: Vec<Vec<u8>>) -> Vec<u8> {
    let mut content = Vec::new();
    for subrecord in subrecords {
        content.extend(subrecord);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(record_type.as_bytes());
    bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&form_id.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&44u16.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend(content);
    bytes
}

fn build_subrecord(record_type: &str, content: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(record_type.as_bytes());
    bytes.extend_from_slice(&(content.len() as u16).to_le_bytes());
    bytes.extend_from_slice(content);
    bytes
}

pub fn write_pex_fixture(path: &Path) {
    write_bytes(path, &pex_fixture_bytes());
}

pub fn write_pex_fixture_with_literals(path: &Path, texts: &[&str]) {
    write_bytes(
        path,
        &pex_fixture_with_literals(texts).write_to_vec().unwrap(),
    );
}

pub fn pex_fixture_bytes() -> Vec<u8> {
    pex_fixture().write_to_vec().unwrap()
}

pub fn pex_entry_text(path: &Path) -> String {
    let written = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(fs::read(path).unwrap()),
    );
    let bundle =
        stringer_pex::read_pex_strings(written, stringer_pex::ReadPexOptions::default()).unwrap();
    bundle.entries()[0].text().to_string()
}

fn pex_fixture() -> PexFile {
    pex_fixture_with_literals(&["Hello world"])
}

fn pex_fixture_with_literals(texts: &[&str]) -> PexFile {
    let mut file = PexFile::new(PexHeader::new_skyrim(0, "Example.psc", "tester", "builder"));
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let instructions = texts
        .iter()
        .map(|text| {
            let id = file.intern(*text).unwrap();
            PexInstruction::new(
                PexOpcode::Assign,
                vec![PexValue::Identifier(tmp), PexValue::String(id)],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    file.objects.push(PexObject {
        name: object,
        parent_class_name: empty,
        documentation_string: empty,
        user_flags: 0,
        auto_state_name: empty,
        variables: Vec::new(),
        properties: Vec::new(),
        states: vec![PexState {
            name: empty,
            functions: vec![PexFunction {
                name: function,
                return_type_name: none,
                documentation_string: empty,
                user_flags: 0,
                is_global: false,
                is_native: false,
                parameters: Vec::new(),
                locals: vec![PexLocal {
                    name: tmp,
                    type_name: string_type,
                }],
                instructions,
            }],
        }],
    });
    file
}
