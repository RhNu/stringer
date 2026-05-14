use stringer_core::{PexCallContext, PexFunctionKind, PexOperandPath};

use crate::{PexOpcode, PexStringId};

#[derive(Debug, Clone)]
pub(crate) struct PexStringFilter {
    rules: Vec<PexFilterRule>,
}

impl PexStringFilter {
    pub(crate) fn default_rules() -> Self {
        Self {
            rules: vec![
                PexFilterRule::EmptySource,
                PexFilterRule::TagListSource,
                PexFilterRule::IdentifierLikeSource,
            ],
        }
    }

    pub(crate) fn evaluate(&self, input: &PexStringFilterInput<'_>) -> Option<PexFilterReason> {
        self.rules.iter().find_map(|rule| rule.evaluate(input))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PexFilterRule {
    EmptySource,
    IdentifierLikeSource,
    TagListSource,
    #[allow(dead_code)]
    FunctionContext,
}

impl PexFilterRule {
    fn evaluate(self, input: &PexStringFilterInput<'_>) -> Option<PexFilterReason> {
        match self {
            Self::EmptySource => input
                .text
                .trim()
                .is_empty()
                .then_some(PexFilterReason::EmptySource),
            Self::IdentifierLikeSource => {
                identifier_like_source(input.text).then_some(PexFilterReason::IdentifierLikeSource)
            }
            Self::TagListSource => {
                tag_list_source(input.text).then_some(PexFilterReason::TagListSource)
            }
            Self::FunctionContext => {
                let _ = (
                    input.path,
                    input.object_name,
                    input.state_name,
                    input.function_name,
                    input.function_kind,
                    input.opcode,
                    input.operand,
                    input.string_id,
                    input.call_context,
                    input.in_concat,
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PexFilterReason {
    EmptySource,
    IdentifierLikeSource,
    TagListSource,
    #[allow(dead_code)]
    FunctionContextRule,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PexStringFilterInput<'a> {
    pub(crate) text: &'a str,
    pub(crate) path: &'a str,
    pub(crate) object_name: &'a str,
    pub(crate) state_name: &'a str,
    pub(crate) function_name: &'a str,
    pub(crate) function_kind: PexFunctionKind,
    pub(crate) opcode: PexOpcode,
    pub(crate) operand: PexOperandPath,
    pub(crate) string_id: PexStringId,
    pub(crate) call_context: Option<&'a PexCallContext>,
    pub(crate) in_concat: bool,
}

fn identifier_like_source(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || text.chars().any(char::is_whitespace) {
        return false;
    }
    if tag_list_source(trimmed) {
        return false;
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.'))
}

fn tag_list_source(text: &str) -> bool {
    let trimmed = text.trim();
    if !trimmed.contains(',') || trimmed.chars().any(char::is_whitespace) {
        return false;
    }
    let mut count = 0;
    for token in trimmed.split(',') {
        if token.is_empty() || !token.chars().all(tag_token_char) {
            return false;
        }
        count += 1;
    }
    count >= 2
}

fn tag_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> PexStringFilterInput<'_> {
        PexStringFilterInput {
            text,
            path: "Data/Scripts/Example.pex",
            object_name: "Example",
            state_name: "",
            function_name: "Run",
            function_kind: PexFunctionKind::Normal,
            opcode: PexOpcode::Assign,
            operand: PexOperandPath::Fixed(1),
            string_id: PexStringId::new(0),
            call_context: None,
            in_concat: false,
        }
    }

    #[test]
    fn empty_source_rule_runs_before_other_rules() {
        let filter = PexStringFilter::default_rules();

        let reason = filter.evaluate(&input("   "));

        assert_eq!(reason, Some(PexFilterReason::EmptySource));
    }
}
