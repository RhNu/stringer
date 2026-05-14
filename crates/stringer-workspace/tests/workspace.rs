use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use camino::Utf8PathBuf;
use serde_json::Value;
use stringer_core::FileAsset;
use stringer_core::Language;
use stringer_pex::{
    PexFile, PexFunction, PexHeader, PexInstruction, PexLocal, PexObject, PexOpcode, PexState,
    PexValue,
};
use stringer_plugin::{GameRelease, StringsFile, StringsKind, write_strings_file};
use stringer_workspace::{
    ExportTranslationsOptions, ImportTranslationsOptions, LoadWorkspaceSettingsOptions,
    WorkspaceSettings, WorkspaceSettingsOverrides, WriteTarget, export_translation_jsonl,
    import_translation_jsonl, load_workspace_settings,
};

#[test]
fn load_settings_reads_toml_config_and_applies_explicit_overrides() {
    let root = TempRoot::new("settings");
    let config = root.path().join("config.toml");
    write_text(
        &config,
        r#"
game_release = "SkyrimSe"
asset_language = "English"
source_locale = "en"
target_locale = "zh-Hans"
"#,
    );

    let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
        config_path: Some(utf8(&config)),
        overrides: WorkspaceSettingsOverrides {
            target_locale: Some("ja".to_string()),
            ..WorkspaceSettingsOverrides::default()
        },
    })
    .unwrap();

    assert_eq!(settings.game_release, GameRelease::SkyrimSe);
    assert_eq!(settings.asset_language, Language::English);
    assert_eq!(settings.source_locale, "en");
    assert_eq!(settings.target_locale, "ja");
}

#[tokio::test]
async fn exports_scaleform_jsonl_with_separate_asset_and_target_languages() {
    let root = TempRoot::new("export-scaleform");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let out = root.path().join("translations.jsonl");

    let summary = export_translation_jsonl(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&out),
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.entries, 1);
    let rows = jsonl_rows(&out);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["schema_version"], 1);
    assert_eq!(
        rows[0]["id"],
        "scaleform:Data/Interface/Translations/MyMod_English.txt:$Title"
    );
    assert_eq!(rows[0]["kind"], "scaleform");
    assert_eq!(
        rows[0]["asset_path"],
        "Data/Interface/Translations/MyMod_English.txt"
    );
    assert_eq!(rows[0]["asset_language"], "English");
    assert_eq!(rows[0]["source_locale"], "en");
    assert_eq!(rows[0]["target_locale"], "zh-Hans");
    assert_eq!(rows[0]["source_text"], "Iron Sword");
}

#[tokio::test]
async fn import_writes_only_changed_override_files_and_leaves_source_unchanged() {
    let root = TempRoot::new("import-scaleform");
    let source = root
        .path()
        .join("Data/Interface/Translations/MyMod_English.txt");
    write_text(&source, "$Title\tIron Sword\n");
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        r#"{"id":"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title","translated_text":"钢剑"}"#,
    );
    let override_root = TempRoot::new("import-scaleform-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    assert_eq!(fs::read_to_string(&source).unwrap(), "$Title\tIron Sword\n");
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t钢剑\n"));
}

#[tokio::test]
async fn import_rejects_duplicate_translated_ids() {
    let root = TempRoot::new("duplicate-id");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        concat!(
            "{\"id\":\"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title\",\"translated_text\":\"A\"}\n",
            "{\"id\":\"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title\",\"translated_text\":\"B\"}\n",
        ),
    );

    let error = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
        settings: settings(),
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("duplicate translation id"));
}

#[tokio::test]
async fn import_rejects_override_root_that_is_the_source_root() {
    let root = TempRoot::new("unsafe-override-root");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        r#"{"id":"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title","translated_text":"Steel Sword"}"#,
    );

    let error = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap_err();

    assert!(error.to_string().contains("override root"));
    assert_eq!(
        fs::read_to_string(
            root.path()
                .join("Data/Interface/Translations/MyMod_English.txt")
        )
        .unwrap(),
        "$Title\tIron Sword\n"
    );
}

#[tokio::test]
async fn import_rejects_unknown_translation_ids() {
    let root = TempRoot::new("unknown-id");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        r#"{"id":"scaleform:Data/Interface/Translations/MyMod_English.txt:$Missing","translated_text":"Steel Sword"}"#,
    );

    let error = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(&root.path().join("override")),
        },
        settings: settings(),
    })
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("does not match any exported entry")
    );
}

#[tokio::test]
async fn import_skips_missing_and_null_translated_text_without_writing_files() {
    let root = TempRoot::new("skip-null");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n$Desc\tSharp blade\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        concat!(
            "{\"id\":\"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title\"}\n",
            "{\"id\":\"scaleform:Data/Interface/Translations/MyMod_English.txt:$Desc\",\"translated_text\":null}\n",
        ),
    );
    let override_root = TempRoot::new("skip-null-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 0);
    assert_eq!(summary.written_files, 0);
    assert!(!override_root.path().join("Data").exists());
}

#[tokio::test]
async fn import_applies_empty_string_translations() {
    let root = TempRoot::new("empty-string");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        r#"{"id":"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title","translated_text":""}"#,
    );
    let override_root = TempRoot::new("empty-string-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    let written = fs::read(
        override_root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
    )
    .unwrap();
    assert!(decode_utf16le_bom(&written).contains("$Title\t\n"));
}

#[tokio::test]
async fn import_same_text_translation_does_not_write_override_files() {
    let root = TempRoot::new("same-text");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Title\tIron Sword\n",
    );
    let translations = root.path().join("translations.jsonl");
    write_text(
        &translations,
        r#"{"id":"scaleform:Data/Interface/Translations/MyMod_English.txt:$Title","translated_text":"Iron Sword"}"#,
    );
    let override_root = TempRoot::new("same-text-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&translations),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 0);
    assert!(!override_root.path().join("Data").exists());
}

#[tokio::test]
async fn exported_ids_normalize_only_path_separators_not_key_text() {
    let root = TempRoot::new("id-normalization");
    write_text(
        &root
            .path()
            .join("Data/Interface/Translations/MyMod_English.txt"),
        "$Path\\Key\tIron Sword\n",
    );
    let out = root.path().join("translations.jsonl");

    export_translation_jsonl(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&out),
        settings: settings(),
    })
    .await
    .unwrap();

    let rows = jsonl_rows(&out);
    assert_eq!(
        rows[0]["id"],
        "scaleform:Data/Interface/Translations/MyMod_English.txt:$Path\\Key"
    );
}

#[tokio::test]
async fn import_updates_localized_plugin_strings_without_copying_unchanged_plugin() {
    let root = TempRoot::new("plugin-import");
    write_bytes(
        &root.path().join("Data/MyMod.esp"),
        &build_localized_plugin(),
    );
    let mut strings = StringsFile::new(StringsKind::Normal, Language::English);
    strings.insert(1, "Iron Sword");
    let strings_asset = write_strings_file(
        "Data/Strings/MyMod_English.STRINGS",
        &strings,
        GameRelease::SkyrimSe,
    )
    .unwrap();
    write_bytes(
        &root.path().join("Data/Strings/MyMod_English.STRINGS"),
        strings_asset.bytes(),
    );

    let export_path = root.path().join("translations.jsonl");
    export_translation_jsonl(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();
    let rows = jsonl_rows(&export_path);
    let plugin_row = rows
        .iter()
        .find(|row| row["kind"] == "plugin")
        .expect("plugin row");
    let import_path = root.path().join("translated.jsonl");
    write_text(
        &import_path,
        &format!(
            "{{\"id\":{},\"translated_text\":\"钢剑\"}}\n",
            serde_json::to_string(plugin_row["id"].as_str().unwrap()).unwrap()
        ),
    );
    let override_root = TempRoot::new("plugin-import-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&import_path),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    assert!(!override_root.path().join("Data/MyMod.esp").exists());
    assert!(
        override_root
            .path()
            .join("Data/Strings/MyMod_English.STRINGS")
            .exists()
    );
}

#[tokio::test]
async fn import_updates_pex_literals_into_override_script() {
    let root = TempRoot::new("pex-import");
    let source = root.path().join("Data/Scripts/Example.pex");
    write_bytes(&source, &pex_fixture().write_to_vec().unwrap());
    let export_path = root.path().join("translations.jsonl");
    export_translation_jsonl(ExportTranslationsOptions {
        root: utf8(root.path()),
        out: utf8(&export_path),
        settings: settings(),
    })
    .await
    .unwrap();
    let rows = jsonl_rows(&export_path);
    let pex_row = rows
        .iter()
        .find(|row| row["kind"] == "pex")
        .expect("pex row");
    let import_path = root.path().join("translated.jsonl");
    write_text(
        &import_path,
        &format!(
            "{{\"id\":{},\"translated_text\":\"你好\"}}\n",
            serde_json::to_string(pex_row["id"].as_str().unwrap()).unwrap()
        ),
    );
    let override_root = TempRoot::new("pex-import-override");

    let summary = import_translation_jsonl(ImportTranslationsOptions {
        root: utf8(root.path()),
        translations: utf8(&import_path),
        target: WriteTarget::OverrideDirectory {
            root: utf8(override_root.path()),
        },
        settings: settings(),
    })
    .await
    .unwrap();

    assert_eq!(summary.applied_entries, 1);
    assert_eq!(summary.written_files, 1);
    let written = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(fs::read(override_root.path().join("Data/Scripts/Example.pex")).unwrap()),
    );
    let bundle =
        stringer_pex::read_pex_strings(written, stringer_pex::ReadPexOptions::default()).unwrap();
    assert_eq!(bundle.entries()[0].text(), "你好");
    assert_eq!(
        fs::read(source).unwrap(),
        pex_fixture().write_to_vec().unwrap()
    );
}

fn settings() -> WorkspaceSettings {
    WorkspaceSettings {
        game_release: GameRelease::SkyrimSe,
        asset_language: Language::English,
        source_locale: "en".to_string(),
        target_locale: "zh-Hans".to_string(),
    }
}

fn jsonl_rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn write_text(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn utf8(path: &Path) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(path.to_path_buf()).unwrap()
}

fn decode_utf16le_bom(bytes: &[u8]) -> String {
    assert!(bytes.starts_with(&[0xFF, 0xFE]));
    let units = bytes[2..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&units).unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
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

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}

fn build_localized_plugin() -> Vec<u8> {
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

fn pex_fixture() -> PexFile {
    let mut file = PexFile::new(PexHeader::new_skyrim(0, "Example.psc", "tester", "builder"));
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let hello = file.intern("hello").unwrap();
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
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::Assign,
                        vec![PexValue::Identifier(tmp), PexValue::String(hello)],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    file
}
