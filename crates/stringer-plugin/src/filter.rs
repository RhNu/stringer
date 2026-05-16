use stringer_core::PluginStringStorage;
use stringer_extraction_filter::{
    ExtractionFilterInput, ExtractionFilterMatch, ExtractionFilterSet, evaluate_builtin,
};

use crate::{LocalizedFieldSource, StringsKind};

#[derive(Debug, Clone)]
pub(crate) struct PluginStringFilter {
    rules: ExtractionFilterSet,
}

impl PluginStringFilter {
    pub(crate) fn with_rules(rules: ExtractionFilterSet) -> Self {
        Self { rules }
    }

    pub(crate) fn evaluate(
        &self,
        input: &PluginStringFilterInput<'_>,
    ) -> Option<ExtractionFilterMatch> {
        if self.rules.is_default() {
            return evaluate_builtin("plugin", input.text);
        }
        self.rules.evaluate(&input.as_extraction_input())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PluginStringFilterInput<'a> {
    pub(crate) text: &'a str,
    pub(crate) path: &'a str,
    pub(crate) record_type: &'a str,
    pub(crate) form_id: u32,
    pub(crate) subrecord: &'a str,
    pub(crate) field_source: LocalizedFieldSource,
    pub(crate) storage: PluginStringStorage,
    pub(crate) strings_kind: StringsKind,
    pub(crate) string_id: Option<u32>,
}

impl PluginStringFilterInput<'_> {
    fn as_extraction_input(&self) -> ExtractionFilterInput {
        let mut input = ExtractionFilterInput::new("plugin", asset_path(self.path), self.text)
            .with_context("path", self.path)
            .with_context("record_type", self.record_type)
            .with_context("form_id", format!("{:#010X}", self.form_id))
            .with_context("subrecord", self.subrecord)
            .with_context("field_source", field_source_name(self.field_source))
            .with_context("storage", storage_name(self.storage))
            .with_context("strings_kind", self.strings_kind.extension());
        if let Some(string_id) = self.string_id {
            input.insert_context("string_id", string_id.to_string());
        }
        input
    }
}

fn asset_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.len() > 5 && normalized[..5].eq_ignore_ascii_case("Data/") {
        return normalized[5..].to_string();
    }
    normalized
}

fn field_source_name(source: LocalizedFieldSource) -> &'static str {
    match source {
        LocalizedFieldSource::Normal => "Normal",
        LocalizedFieldSource::Dl => "DL",
        LocalizedFieldSource::Il => "IL",
    }
}

fn storage_name(storage: PluginStringStorage) -> &'static str {
    match storage {
        PluginStringStorage::Localized => "localized",
        PluginStringStorage::Embedded => "embedded",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> PluginStringFilterInput<'_> {
        PluginStringFilterInput {
            text,
            path: "Data/MyMod.esp",
            record_type: "WEAP",
            form_id: 0x800,
            subrecord: "FULL",
            field_source: LocalizedFieldSource::Normal,
            storage: PluginStringStorage::Localized,
            strings_kind: StringsKind::Normal,
            string_id: Some(1),
        }
    }

    #[test]
    fn empty_source_rule_filters_whitespace_text() {
        let filter = PluginStringFilter::with_rules(ExtractionFilterSet::default());

        let reason = filter.evaluate(&input("  \t "));

        assert_eq!(reason.unwrap().rule_id(), "core.empty_source");
    }
}
