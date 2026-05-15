use stringer_core::PluginStringStorage;

use crate::{LocalizedFieldSource, StringsKind};

#[derive(Debug, Clone)]
pub(crate) struct PluginStringFilter {
    rules: Vec<PluginFilterRule>,
}

impl PluginStringFilter {
    pub(crate) fn default_rules() -> Self {
        Self {
            rules: vec![PluginFilterRule::EmptySource],
        }
    }

    pub(crate) fn evaluate(
        &self,
        input: &PluginStringFilterInput<'_>,
    ) -> Option<PluginFilterReason> {
        self.rules.iter().find_map(|rule| rule.evaluate(input))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginFilterRule {
    EmptySource,
    #[allow(dead_code)]
    RecordContext,
}

impl PluginFilterRule {
    fn evaluate(self, input: &PluginStringFilterInput<'_>) -> Option<PluginFilterReason> {
        match self {
            Self::EmptySource => input
                .text
                .trim()
                .is_empty()
                .then_some(PluginFilterReason::EmptySource),
            Self::RecordContext => {
                let _ = (
                    input.path,
                    input.record_type,
                    input.form_id,
                    input.subrecord,
                    input.field_source,
                    input.storage,
                    input.strings_kind,
                    input.string_id,
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginFilterReason {
    EmptySource,
    #[allow(dead_code)]
    RecordContextRule,
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
        let filter = PluginStringFilter::default_rules();

        let reason = filter.evaluate(&input("  \t "));

        assert_eq!(reason, Some(PluginFilterReason::EmptySource));
    }
}
