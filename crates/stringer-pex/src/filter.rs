use stringer_core::{PexCallContext, PexFunctionKind, PexOperandPath};
use stringer_extraction_filter::{
    ExtractionFilterInput, ExtractionFilterMatch, ExtractionFilterSet, evaluate_builtin,
};

use crate::{PexOpcode, PexStringId};

#[derive(Debug, Clone)]
pub(crate) struct PexStringFilter {
    rules: ExtractionFilterSet,
}

impl PexStringFilter {
    pub(crate) fn with_rules(rules: ExtractionFilterSet) -> Self {
        Self { rules }
    }

    pub(crate) fn evaluate(
        &self,
        input: &PexStringFilterInput<'_>,
    ) -> Option<ExtractionFilterMatch> {
        if self.rules.is_default() {
            return evaluate_builtin("pex", input.text);
        }
        self.rules.evaluate(&input.as_extraction_input())
    }
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

impl PexStringFilterInput<'_> {
    fn as_extraction_input(&self) -> ExtractionFilterInput {
        let mut input = ExtractionFilterInput::new("pex", asset_path(self.path), self.text)
            .with_context("path", self.path)
            .with_context("object", self.object_name)
            .with_context("state", self.state_name)
            .with_context("function", self.function_name)
            .with_context("function_kind", format!("{:?}", self.function_kind))
            .with_context("opcode", self.opcode.name())
            .with_context("operand", operand_name(self.operand))
            .with_context("string_id", self.string_id.index().to_string())
            .with_context("in_concat", self.in_concat.to_string());
        if let Some(call_context) = self.call_context {
            input.insert_context("call_opcode", call_context.opcode.clone());
            if let Some(target) = &call_context.target {
                input.insert_context("call_target", target.clone());
            }
            if let Some(member) = &call_context.member {
                input.insert_context("call_member", member.clone());
            }
        }
        input
    }
}

fn operand_name(operand: PexOperandPath) -> String {
    match operand {
        PexOperandPath::Fixed(index) => format!("fixed-{index}"),
        PexOperandPath::Variadic(index) => format!("variadic-{index}"),
    }
}

fn asset_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    if normalized.len() > 5 && normalized[..5].eq_ignore_ascii_case("Data/") {
        return normalized[5..].to_string();
    }
    normalized
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
        let filter = PexStringFilter::with_rules(ExtractionFilterSet::default());

        let reason = filter.evaluate(&input("   "));

        assert_eq!(reason.unwrap().rule_id(), "core.empty_source");
    }
}
