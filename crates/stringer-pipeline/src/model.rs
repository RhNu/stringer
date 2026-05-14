use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineEntryKind {
    Plugin,
    Strings,
    Scaleform,
    Pex,
}

impl PipelineEntryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Strings => "strings",
            Self::Scaleform => "scaleform",
            Self::Pex => "pex",
        }
    }

    pub fn from_package_kind(value: &str) -> Option<Self> {
        match value {
            "plugin" => Some(Self::Plugin),
            "strings" => Some(Self::Strings),
            "scaleform" => Some(Self::Scaleform),
            "pex" => Some(Self::Pex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Collect,
    Annotate,
    PreTranslate,
    MemoryApply,
    PostTranslate,
    Validate,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineAnnotation {
    kind: String,
    id: String,
    layer: String,
    confidence: f32,
    #[serde(rename = "match")]
    match_kind: String,
    processor: String,
    #[serde(default = "empty_payload")]
    payload: Value,
}

impl PipelineAnnotation {
    pub fn new(
        kind: impl Into<String>,
        id: impl Into<String>,
        layer: impl Into<String>,
        confidence: f32,
        match_kind: impl Into<String>,
        processor: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
            layer: layer.into(),
            confidence,
            match_kind: match_kind.into(),
            processor: processor.into(),
            payload,
        }
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn layer(&self) -> &str {
        &self.layer
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    pub fn match_kind(&self) -> &str {
        &self.match_kind
    }

    pub fn processor(&self) -> &str {
        &self.processor
    }

    pub fn payload(&self) -> &Value {
        &self.payload
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineDiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl PipelineDiagnosticSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineDiagnostic {
    severity: PipelineDiagnosticSeverity,
    code: String,
    message: String,
    entry_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_id: Option<String>,
}

impl PipelineDiagnostic {
    pub fn new(
        severity: PipelineDiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
        entry_id: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            entry_id: entry_id.into(),
            layer: None,
            rule_id: None,
        }
    }

    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.layer = Some(layer.into());
        self
    }

    pub fn with_rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }

    pub fn severity(&self) -> PipelineDiagnosticSeverity {
        self.severity
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn entry_id(&self) -> &str {
        &self.entry_id
    }

    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }

    pub fn rule_id(&self) -> Option<&str> {
        self.rule_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineEntry {
    id: String,
    kind: PipelineEntryKind,
    source_text: String,
    translated_text: Option<String>,
    source_locale: String,
    target_locale: String,
    asset_path: String,
    context: BTreeMap<String, String>,
    annotations: Vec<PipelineAnnotation>,
    diagnostics: Vec<PipelineDiagnostic>,
}

impl PipelineEntry {
    pub fn new(
        id: impl Into<String>,
        kind: PipelineEntryKind,
        source_text: impl Into<String>,
        source_locale: impl Into<String>,
        target_locale: impl Into<String>,
        asset_path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            source_text: source_text.into(),
            translated_text: None,
            source_locale: source_locale.into(),
            target_locale: target_locale.into(),
            asset_path: asset_path.into(),
            context: BTreeMap::new(),
            annotations: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> PipelineEntryKind {
        self.kind
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn translated_text(&self) -> Option<&str> {
        self.translated_text.as_deref()
    }

    pub fn set_translated_text(&mut self, text: impl Into<String>) {
        self.translated_text = Some(text.into());
    }

    pub fn source_locale(&self) -> &str {
        &self.source_locale
    }

    pub fn target_locale(&self) -> &str {
        &self.target_locale
    }

    pub fn asset_path(&self) -> &str {
        &self.asset_path
    }

    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    pub fn insert_context(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.context.insert(key.into(), value.into())
    }

    pub fn annotations(&self) -> &[PipelineAnnotation] {
        &self.annotations
    }

    pub fn diagnostics(&self) -> &[PipelineDiagnostic] {
        &self.diagnostics
    }

    pub fn add_annotation(&mut self, annotation: PipelineAnnotation) {
        self.annotations.push(annotation);
    }

    pub fn set_annotations(&mut self, annotations: Vec<PipelineAnnotation>) {
        self.annotations = annotations;
    }

    pub fn add_diagnostic(&mut self, diagnostic: PipelineDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<PipelineDiagnostic>) {
        self.diagnostics = diagnostics;
    }

    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    pub fn clear_annotations_from_processors(&mut self, processors: &[&str]) {
        self.annotations
            .retain(|annotation| !processors.contains(&annotation.processor()));
    }

    pub fn into_annotations_and_diagnostics(
        self,
    ) -> (
        Option<String>,
        Vec<PipelineAnnotation>,
        Vec<PipelineDiagnostic>,
    ) {
        (self.translated_text, self.annotations, self.diagnostics)
    }

    pub(crate) fn entry_value(&self, key: &str) -> Option<&str> {
        match key {
            "kind" => Some(self.kind.as_str()),
            "source_locale" => Some(&self.source_locale),
            "target_locale" => Some(&self.target_locale),
            "asset_path" => Some(&self.asset_path),
            _ => self.context.get(key).map(String::as_str),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PipelineOptions {
    pub allow_memory_auto_fill: bool,
    pub execute_replacements: bool,
    pub memory_auto_fill_threshold: f32,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            allow_memory_auto_fill: false,
            execute_replacements: false,
            memory_auto_fill_threshold: 0.95,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PipelineReport {
    pub entries: usize,
    pub annotations: usize,
    pub diagnostics: Vec<PipelineDiagnostic>,
    pub auto_filled: usize,
    pub skipped: usize,
}

impl PipelineReport {
    pub fn diagnostics_by_severity(&self, severity: &str) -> Vec<&PipelineDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity().as_str() == severity)
            .collect()
    }
}

pub(crate) fn annotation_payload(pairs: &[(&str, &str)]) -> Value {
    let mut value = serde_json::Map::new();
    for (key, item) in pairs {
        value.insert((*key).to_string(), json!(item));
    }
    Value::Object(value)
}

fn empty_payload() -> Value {
    Value::Object(serde_json::Map::new())
}
