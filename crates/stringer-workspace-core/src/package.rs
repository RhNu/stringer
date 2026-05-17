use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use stringer_core::{
    PexOperandPath, PluginStringStorage, StringEntry, StringEntryContext, StringEntrySource,
    StringEntryView,
};
use stringer_extraction_filter::{ExtractionFilterConfig, ExtractionFilterSet};
use stringer_pipeline::{PipelineAnnotation, PipelineDiagnostic};

use crate::WorkspaceCoreError;
use crate::fsutil::{replace_file, temp_path};
use crate::lock::unix_ms;
use crate::settings::{
    WorkspaceSettings, game_release_name, language_name, parse_game_release_name,
    parse_language_name,
};

pub const SCHEMA_VERSION: u32 = 4;

const WORKSPACE_FILE: &str = "workspace.json";
const LEGACY_MANIFEST_FILE: &str = "manifest.json";
const BATCHES_DIR: &str = "batches";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationManifest {
    pub schema_version: u32,
    pub kind: String,
    pub source_root: String,
    pub game_release: String,
    pub asset_language: String,
    pub source_locale: String,
    pub target_locale: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extraction_filters: Option<ExtractionFilterConfig>,
    pub files: Vec<TranslationManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationManifestFile {
    pub path: String,
    pub kind: String,
    pub asset_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRecord {
    pub id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation_meta: Option<TranslationMeta>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "hints")]
    pub hints: Vec<PipelineAnnotation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<PipelineDiagnostic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_unix_ms: Option<u128>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackagedTranslationRecord {
    pub file: TranslationFileKey,
    pub record: TranslationRecord,
}

#[derive(Debug, Clone)]
pub struct TranslationPackageRecords {
    pub settings: WorkspaceSettings,
    pub files: Vec<TranslationPackageFileRecords>,
}

#[derive(Debug, Clone)]
pub struct TranslationPackageFileRecords {
    pub manifest_file: TranslationManifestFile,
    pub records: Vec<TranslationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranslationFileKey {
    kind: String,
    asset_path: String,
    group: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslationPatchRecord {
    id: String,
    #[serde(default)]
    translation: Option<String>,
}

pub fn packaged_record_from_entry(
    entry: &impl StringEntryView,
    _settings: &WorkspaceSettings,
) -> PackagedTranslationRecord {
    let entry = entry.string_entry();
    let (file, source_context) = source_fields(entry.source());
    let mut context = context_values(entry.context());
    for (key, value) in source_context {
        context.entry(key).or_insert(value);
    }
    PackagedTranslationRecord {
        file,
        record: TranslationRecord {
            id: external_entry_id(entry),
            source: entry.text().to_string(),
            translation: None,
            translation_meta: None,
            context,
            hints: Vec::new(),
            diagnostics: Vec::new(),
        },
    }
}

pub fn write_translation_package(
    path: &Utf8Path,
    source_root: &Utf8Path,
    settings: &WorkspaceSettings,
    records: &[PackagedTranslationRecord],
) -> Result<(), WorkspaceCoreError> {
    fs::create_dir_all(path).map_err(|source| WorkspaceCoreError::WriteFile {
        path: path.to_owned(),
        source,
    })?;
    let batches = path.join(BATCHES_DIR);
    if batches.exists() {
        fs::remove_dir_all(&batches).map_err(|source| WorkspaceCoreError::WriteFile {
            path: batches.clone(),
            source,
        })?;
    }
    fs::create_dir_all(&batches).map_err(|source| WorkspaceCoreError::WriteFile {
        path: batches,
        source,
    })?;

    let mut groups = BTreeMap::<TranslationFileKey, Vec<TranslationRecord>>::new();
    for record in records {
        groups
            .entry(record.file.clone())
            .or_default()
            .push(record.record.clone());
    }

    let mut files = Vec::new();
    for (key, mut records) in groups {
        records.sort_by(|left, right| left.id.cmp(&right.id));
        let manifest_file = key.manifest_file();
        let output_path = path.join(&manifest_file.path);
        write_records_jsonl(&output_path, &records)?;
        files.push(manifest_file);
    }

    let manifest = TranslationManifest {
        schema_version: SCHEMA_VERSION,
        kind: "stringer.workspace".to_string(),
        source_root: source_root.as_str().replace('\\', "/"),
        game_release: game_release_name(settings.game_release).to_string(),
        asset_language: language_name(settings.asset_language).to_string(),
        source_locale: settings.source_locale.clone(),
        target_locale: settings.target_locale.clone(),
        extraction_filters: settings.extraction_filters.config().cloned(),
        files,
    };
    let legacy = path.join(LEGACY_MANIFEST_FILE);
    if legacy.exists() {
        fs::remove_file(&legacy).map_err(|source| WorkspaceCoreError::WriteFile {
            path: legacy,
            source,
        })?;
    }
    write_manifest(&path.join(WORKSPACE_FILE), &manifest)
}

pub fn read_translation_package(
    path: &Utf8Path,
) -> Result<(WorkspaceSettings, Utf8PathBuf, BTreeMap<String, String>), WorkspaceCoreError> {
    let manifest = read_translation_manifest(path)?;
    let settings = settings_from_manifest(&manifest)?;
    let source_root = source_root_from_manifest(&manifest)?;

    let mut seen_files = BTreeSet::new();
    let mut seen_ids = BTreeSet::new();
    let mut patches = BTreeMap::new();
    for file in manifest.files {
        let entry_path = package_entry_path(path, &file.path)?;
        let normalized_file = normalize_path(&file.path);
        if !seen_files.insert(normalized_file) {
            return Err(WorkspaceCoreError::InvalidTranslationPackagePath {
                path: file.path,
                message: "workspace.json lists the same entry file more than once".to_string(),
            });
        }
        read_patch_file(&entry_path, &mut seen_ids, &mut patches)?;
    }

    Ok((settings, source_root, patches))
}

pub fn read_workspace_settings(path: &Utf8Path) -> Result<WorkspaceSettings, WorkspaceCoreError> {
    settings_from_manifest(&read_translation_manifest(path)?)
}

pub fn read_workspace_source_root(path: &Utf8Path) -> Result<Utf8PathBuf, WorkspaceCoreError> {
    source_root_from_manifest(&read_translation_manifest(path)?)
}

pub fn read_translation_package_records(
    path: &Utf8Path,
) -> Result<TranslationPackageRecords, WorkspaceCoreError> {
    read_translation_package_records_filtered(path, None)
}

pub fn read_translation_package_records_filtered(
    path: &Utf8Path,
    file_filter: Option<&str>,
) -> Result<TranslationPackageRecords, WorkspaceCoreError> {
    let manifest = read_translation_manifest(path)?;
    let settings = settings_from_manifest(&manifest)?;
    let manifest_files = validate_manifest_files(path, manifest.files)?;
    let mut files = Vec::new();
    visit_manifest_records(
        path,
        manifest_files,
        file_filter,
        |manifest_file, records| {
            files.push(TranslationPackageFileRecords {
                manifest_file: manifest_file.clone(),
                records,
            });
            Ok(())
        },
    )?;

    Ok(TranslationPackageRecords { settings, files })
}

pub fn visit_translation_package_records_filtered<F>(
    path: &Utf8Path,
    file_filter: Option<&str>,
    mut visit: F,
) -> Result<WorkspaceSettings, WorkspaceCoreError>
where
    F: FnMut(&TranslationManifestFile, TranslationRecord) -> Result<(), WorkspaceCoreError>,
{
    let manifest = read_translation_manifest(path)?;
    let settings = settings_from_manifest(&manifest)?;
    let manifest_files = validate_manifest_files(path, manifest.files)?;
    visit_manifest_records_streaming(path, manifest_files, file_filter, &mut visit)?;
    Ok(settings)
}

pub fn read_translation_manifest_files(
    path: &Utf8Path,
) -> Result<Vec<TranslationManifestFile>, WorkspaceCoreError> {
    let manifest = read_translation_manifest(path)?;
    validate_manifest_files(path, manifest.files)
}

fn visit_manifest_records<F>(
    path: &Utf8Path,
    manifest_files: Vec<TranslationManifestFile>,
    file_filter: Option<&str>,
    mut visit: F,
) -> Result<(), WorkspaceCoreError>
where
    F: FnMut(&TranslationManifestFile, Vec<TranslationRecord>) -> Result<(), WorkspaceCoreError>,
{
    let normalized_filter = file_filter.map(normalize_path);
    let mut found_filter = normalized_filter.is_none();
    let mut seen_ids = BTreeSet::new();
    for manifest_file in manifest_files {
        let normalized_file = normalize_path(&manifest_file.path);
        let should_read = normalized_filter
            .as_deref()
            .is_none_or(|expected| expected == normalized_file);
        if !should_read {
            continue;
        }
        found_filter = true;
        let entry_path = package_entry_path(path, &manifest_file.path)?;
        let records = read_record_file(&entry_path, &mut seen_ids)?;
        visit(&manifest_file, records)?;
    }
    if !found_filter {
        return Err(WorkspaceCoreError::InvalidTranslationPackagePath {
            path: file_filter.unwrap_or_default().to_string(),
            message: "entry file is not listed in workspace.json".to_string(),
        });
    }
    Ok(())
}

fn visit_manifest_records_streaming<F>(
    path: &Utf8Path,
    manifest_files: Vec<TranslationManifestFile>,
    file_filter: Option<&str>,
    visit: &mut F,
) -> Result<(), WorkspaceCoreError>
where
    F: FnMut(&TranslationManifestFile, TranslationRecord) -> Result<(), WorkspaceCoreError>,
{
    let normalized_filter = file_filter.map(normalize_path);
    let mut found_filter = normalized_filter.is_none();
    let mut seen_ids = BTreeSet::new();
    for manifest_file in manifest_files {
        let normalized_file = normalize_path(&manifest_file.path);
        let should_read = normalized_filter
            .as_deref()
            .is_none_or(|expected| expected == normalized_file);
        if !should_read {
            continue;
        }
        found_filter = true;
        let entry_path = package_entry_path(path, &manifest_file.path)?;
        visit_record_file(&entry_path, &manifest_file, &mut seen_ids, visit)?;
    }
    if !found_filter {
        return Err(WorkspaceCoreError::InvalidTranslationPackagePath {
            path: file_filter.unwrap_or_default().to_string(),
            message: "entry file is not listed in workspace.json".to_string(),
        });
    }
    Ok(())
}

pub fn read_translation_manifest(
    path: &Utf8Path,
) -> Result<TranslationManifest, WorkspaceCoreError> {
    let manifest_path = path.join(WORKSPACE_FILE);
    let manifest = read_manifest(path, &manifest_path)?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(WorkspaceCoreError::UnsupportedTranslationSchema {
            path: manifest_path,
            version: manifest.schema_version,
        });
    }

    Ok(manifest)
}

fn validate_manifest_files(
    root: &Utf8Path,
    files: Vec<TranslationManifestFile>,
) -> Result<Vec<TranslationManifestFile>, WorkspaceCoreError> {
    let mut seen_files = BTreeSet::new();
    for file in &files {
        let normalized_file = normalize_path(&file.path);
        if !seen_files.insert(normalized_file) {
            return Err(WorkspaceCoreError::InvalidTranslationPackagePath {
                path: file.path.clone(),
                message: "workspace.json lists the same entry file more than once".to_string(),
            });
        }
        package_entry_path(root, &file.path)?;
    }
    Ok(files)
}

pub fn write_translation_package_records(
    path: &Utf8Path,
    package: &TranslationPackageRecords,
) -> Result<(), WorkspaceCoreError> {
    for file in &package.files {
        let entry_path = package_entry_path(path, &file.manifest_file.path)?;
        write_records_jsonl(&entry_path, &file.records)?;
    }
    Ok(())
}

fn write_manifest(
    path: &Utf8Path,
    manifest: &TranslationManifest,
) -> Result<(), WorkspaceCoreError> {
    let temp = temp_path(path, unix_ms().to_string());
    let file = fs::File::create(&temp).map_err(|source| WorkspaceCoreError::WriteFile {
        path: temp.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, manifest).map_err(|source| {
        WorkspaceCoreError::Json {
            path: temp.clone(),
            source,
        }
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| WorkspaceCoreError::WriteFile {
            path: temp.clone(),
            source,
        })?;
    writer
        .flush()
        .map_err(|source| WorkspaceCoreError::WriteFile {
            path: temp.clone(),
            source,
        })?;
    replace_file(&temp, path)
}

fn read_manifest(
    root: &Utf8Path,
    path: &Utf8Path,
) -> Result<TranslationManifest, WorkspaceCoreError> {
    let text = fs::read_to_string(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    });
    let text = match text {
        Ok(text) => text,
        Err(WorkspaceCoreError::ReadFile { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound
                && root.join(LEGACY_MANIFEST_FILE).exists() =>
        {
            return Err(WorkspaceCoreError::LegacyTranslationWorkspace {
                path: root.to_owned(),
            });
        }
        Err(error) => return Err(error),
    };
    serde_json::from_str(&text).map_err(|source| WorkspaceCoreError::Json {
        path: path.to_owned(),
        source,
    })
}

fn settings_from_manifest(
    manifest: &TranslationManifest,
) -> Result<WorkspaceSettings, WorkspaceCoreError> {
    Ok(WorkspaceSettings {
        game_release: parse_game_release_name(&manifest.game_release)?,
        asset_language: parse_language_name(&manifest.asset_language)?,
        source_locale: required_manifest_setting(manifest.source_locale.clone(), "source_locale")?,
        target_locale: required_manifest_setting(manifest.target_locale.clone(), "target_locale")?,
        global_knowledge_root: None,
        extraction_filters: manifest
            .extraction_filters
            .clone()
            .map(ExtractionFilterSet::from_config)
            .transpose()?
            .unwrap_or_default(),
    })
}

fn source_root_from_manifest(
    manifest: &TranslationManifest,
) -> Result<Utf8PathBuf, WorkspaceCoreError> {
    let value = required_manifest_setting(manifest.source_root.clone(), "source_root")?;
    let path = Utf8PathBuf::from(value);
    if !path.is_absolute() {
        return Err(WorkspaceCoreError::InvalidTranslationPackagePath {
            path: path.to_string(),
            message: "workspace source_root must be absolute".to_string(),
        });
    }
    Ok(path)
}

fn write_records_jsonl(
    path: &Utf8Path,
    records: &[TranslationRecord],
) -> Result<(), WorkspaceCoreError> {
    if let Some(parent) = path.parent()
        && !parent.as_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| WorkspaceCoreError::WriteFile {
            path: parent.to_owned(),
            source,
        })?;
    }
    let temp = temp_path(path, unix_ms().to_string());
    let file = fs::File::create(&temp).map_err(|source| WorkspaceCoreError::WriteFile {
        path: temp.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record).map_err(|source| {
            WorkspaceCoreError::JsonLine {
                path: temp.clone(),
                line: 0,
                source,
            }
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| WorkspaceCoreError::WriteFile {
                path: temp.clone(),
                source,
            })?;
    }
    writer
        .flush()
        .map_err(|source| WorkspaceCoreError::WriteFile {
            path: temp.clone(),
            source,
        })?;
    replace_file(&temp, path)
}

fn read_patch_file(
    path: &Utf8Path,
    seen_ids: &mut BTreeSet<String>,
    patches: &mut BTreeMap<String, String>,
) -> Result<(), WorkspaceCoreError> {
    let file = fs::File::open(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let reader = BufReader::new(file);
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| WorkspaceCoreError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let patch: TranslationPatchRecord =
            serde_json::from_str(&line).map_err(|source| WorkspaceCoreError::JsonLine {
                path: path.to_owned(),
                line: line_number,
                source,
            })?;
        if !seen_ids.insert(patch.id.clone()) {
            return Err(WorkspaceCoreError::DuplicateTranslationId {
                path: path.to_owned(),
                id: patch.id,
            });
        }
        let Some(translation) = patch.translation else {
            continue;
        };
        patches.insert(patch.id, translation);
    }
    Ok(())
}

fn read_record_file(
    path: &Utf8Path,
    seen_ids: &mut BTreeSet<String>,
) -> Result<Vec<TranslationRecord>, WorkspaceCoreError> {
    let file = fs::File::open(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| WorkspaceCoreError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record: TranslationRecord =
            serde_json::from_str(&line).map_err(|source| WorkspaceCoreError::JsonLine {
                path: path.to_owned(),
                line: line_number,
                source,
            })?;
        if !seen_ids.insert(record.id.clone()) {
            return Err(WorkspaceCoreError::DuplicateTranslationId {
                path: path.to_owned(),
                id: record.id,
            });
        }
        records.push(record);
    }
    Ok(records)
}

fn visit_record_file<F>(
    path: &Utf8Path,
    manifest_file: &TranslationManifestFile,
    seen_ids: &mut BTreeSet<String>,
    visit: &mut F,
) -> Result<(), WorkspaceCoreError>
where
    F: FnMut(&TranslationManifestFile, TranslationRecord) -> Result<(), WorkspaceCoreError>,
{
    let file = fs::File::open(path).map_err(|source| WorkspaceCoreError::ReadFile {
        path: path.to_owned(),
        source,
    })?;
    let reader = BufReader::new(file);
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|source| WorkspaceCoreError::ReadFile {
            path: path.to_owned(),
            source,
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let record: TranslationRecord =
            serde_json::from_str(&line).map_err(|source| WorkspaceCoreError::JsonLine {
                path: path.to_owned(),
                line: line_number,
                source,
            })?;
        if !seen_ids.insert(record.id.clone()) {
            return Err(WorkspaceCoreError::DuplicateTranslationId {
                path: path.to_owned(),
                id: record.id,
            });
        }
        visit(manifest_file, record)?;
    }
    Ok(())
}

fn package_entry_path(root: &Utf8Path, path: &str) -> Result<Utf8PathBuf, WorkspaceCoreError> {
    let relative = Utf8Path::new(path);
    if relative.is_absolute() {
        return Err(invalid_package_path(
            path,
            "entry file path must be relative",
        ));
    }

    let mut components = relative.components();
    let Some(Utf8Component::Normal(first)) = components.next() else {
        return Err(invalid_package_path(
            path,
            "entry file path must not be empty",
        ));
    };
    if first != "entries" {
        return Err(invalid_package_path(
            path,
            "entry file path must start with entries",
        ));
    }
    for component in components {
        if !matches!(component, Utf8Component::Normal(_)) {
            return Err(invalid_package_path(
                path,
                "entry file path must not contain current or parent components",
            ));
        }
    }

    Ok(root.join(relative))
}

fn invalid_package_path(path: &str, message: impl Into<String>) -> WorkspaceCoreError {
    WorkspaceCoreError::InvalidTranslationPackagePath {
        path: path.to_string(),
        message: message.into(),
    }
}

fn required_manifest_setting(
    value: String,
    name: &'static str,
) -> Result<String, WorkspaceCoreError> {
    if value.trim().is_empty() {
        return Err(WorkspaceCoreError::InvalidSetting { name, value });
    }
    Ok(value)
}

fn source_fields(source: &StringEntrySource) -> (TranslationFileKey, BTreeMap<String, String>) {
    match source {
        StringEntrySource::Plugin(metadata) => {
            let mut context = BTreeMap::new();
            context.insert("record_type".to_string(), metadata.record_type.clone());
            context.insert("form_id".to_string(), format!("{:#010X}", metadata.form_id));
            context.insert("subrecord".to_string(), metadata.subrecord.clone());
            context.insert("strings_kind".to_string(), metadata.strings_kind.clone());
            context.insert("field_source".to_string(), metadata.field_source.clone());
            context.insert(
                "storage".to_string(),
                plugin_storage_name(metadata.storage).to_string(),
            );
            if let Some(string_id) = metadata.string_id {
                context.insert("string_id".to_string(), string_id.to_string());
            }
            (
                TranslationFileKey {
                    kind: "plugin".to_string(),
                    asset_path: external_asset_path(metadata.path.as_str()),
                    group: Some(metadata.record_type.clone()),
                },
                context,
            )
        }
        StringEntrySource::Pex(metadata) => {
            let mut context = BTreeMap::new();
            context.insert("object".to_string(), metadata.object.clone());
            context.insert("state".to_string(), metadata.state.clone());
            context.insert("function".to_string(), metadata.function.clone());
            context.insert(
                "function_kind".to_string(),
                format!("{:?}", metadata.function_kind),
            );
            context.insert(
                "instruction_index".to_string(),
                metadata.instruction_index.to_string(),
            );
            context.insert("opcode".to_string(), metadata.opcode.clone());
            context.insert("operand".to_string(), pex_operand_name(metadata.operand));
            context.insert("string_id".to_string(), metadata.string_id.to_string());
            (
                TranslationFileKey {
                    kind: "pex".to_string(),
                    asset_path: external_asset_path(metadata.path.as_str()),
                    group: None,
                },
                context,
            )
        }
        StringEntrySource::Scaleform(metadata) => {
            let mut context = BTreeMap::new();
            if let Some(key) = &metadata.key {
                context.insert("key".to_string(), key.clone());
            }
            (
                TranslationFileKey {
                    kind: "scaleform".to_string(),
                    asset_path: external_asset_path(metadata.path.as_str()),
                    group: None,
                },
                context,
            )
        }
    }
}

impl TranslationFileKey {
    fn manifest_file(&self) -> TranslationManifestFile {
        TranslationManifestFile {
            path: self.package_path(),
            kind: self.kind.clone(),
            asset_path: self.asset_path.clone(),
            group: self.group.clone(),
        }
    }

    fn package_path(&self) -> String {
        match self.kind.as_str() {
            "plugin" => format!(
                "entries/plugin/{}/{}.jsonl",
                self.asset_path,
                self.group.as_deref().unwrap_or("records")
            ),
            "pex" => format!("entries/pex/{}.jsonl", self.asset_path),
            "scaleform" => format!("entries/scaleform/{}.jsonl", self.asset_path),
            _ => format!("entries/{}/{}.jsonl", self.kind, self.asset_path),
        }
    }
}

fn context_values(context: &StringEntryContext) -> BTreeMap<String, String> {
    context.values().clone()
}

fn plugin_storage_name(storage: PluginStringStorage) -> &'static str {
    match storage {
        PluginStringStorage::Localized => "localized",
        PluginStringStorage::Embedded => "embedded",
    }
}

fn pex_operand_name(operand: PexOperandPath) -> String {
    match operand {
        PexOperandPath::Fixed(index) => format!("fixed-{index}"),
        PexOperandPath::Variadic(index) => format!("variadic-{index}"),
    }
}

pub fn external_entry_id(entry: &StringEntry) -> String {
    match entry.source() {
        StringEntrySource::Plugin(metadata) => {
            replace_path_segment(entry.id(), "plugin:", metadata.path.as_str())
        }
        StringEntrySource::Pex(metadata) => {
            replace_path_segment(entry.id(), "pex:", metadata.path.as_str())
        }
        StringEntrySource::Scaleform(metadata) => {
            replace_path_segment(entry.id(), "scaleform:", metadata.path.as_str())
        }
    }
}

fn replace_path_segment(raw_id: &str, prefix: &str, path: &str) -> String {
    let Some(rest) = raw_id.strip_prefix(prefix) else {
        return raw_id.to_string();
    };
    for candidate in [
        path.to_string(),
        path.replace('/', "\\"),
        path.replace('\\', "/"),
    ] {
        if let Some(tail) = rest.strip_prefix(&candidate) {
            return format!("{prefix}{}{tail}", external_asset_path(path));
        }
    }
    raw_id.to_string()
}

fn external_asset_path(value: &str) -> String {
    let canonical = canonical_path(value);
    if canonical.len() > 5 && canonical[..5].eq_ignore_ascii_case("Data/") {
        return canonical[5..].to_string();
    }
    canonical
}

fn canonical_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}
