#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const BUILTIN_RULE_IDS: &[&str] = &[
    "core.empty_source",
    "pex.identifier_like_source",
    "pex.tag_list_source",
];

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ExtractionFilterError {
    #[error("invalid extraction filter rule id `{id}`")]
    InvalidRuleId { id: String },

    #[error("duplicate extraction filter rule `{id}`")]
    DuplicateRule { id: String },

    #[error("extraction filter rule `{id}` must define `when`")]
    MissingExpression { id: String },

    #[error("invalid extraction filter field `{field}` in rule `{id}`")]
    InvalidField { id: String, field: String },

    #[error("empty extraction filter group `{group}` in rule `{id}`")]
    EmptyGroup { id: String, group: &'static str },

    #[error("invalid extraction filter operator `{operator}` in rule `{id}`")]
    InvalidOperator { id: String, operator: String },

    #[error("extraction filter operator `{operator}` in rule `{id}` requires `value`")]
    MissingValue { id: String, operator: String },

    #[error("extraction filter operator `{operator}` in rule `{id}` must not define `value`")]
    UnexpectedValue { id: String, operator: String },

    #[error(
        "invalid extraction filter value for operator `{operator}` in rule `{id}`: expected {expected}"
    )]
    InvalidValue {
        id: String,
        operator: String,
        expected: &'static str,
    },

    #[error("invalid extraction filter regex in rule `{id}`: {message}")]
    InvalidRegex { id: String, message: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionFilterConfig {
    #[serde(default)]
    pub rules: Vec<ExtractionFilterRuleConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionFilterRuleConfig {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub when: Option<ExtractionFilterExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(untagged)]
pub enum ExtractionFilterExpression {
    All {
        all: Vec<ExtractionFilterExpression>,
    },
    Any {
        any: Vec<ExtractionFilterExpression>,
    },
    Not {
        not: Box<ExtractionFilterExpression>,
    },
    Condition {
        field: String,
        op: String,
        #[serde(default)]
        value: Option<ExtractionFilterValue>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionFilterOperator {
    Eq,
    Ne,
    In,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    Exists,
    IsEmpty,
    IdentifierLike,
    TagList,
}

impl ExtractionFilterOperator {
    fn name(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::In => "in",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::Regex => "regex",
            Self::Exists => "exists",
            Self::IsEmpty => "is_empty",
            Self::IdentifierLike => "identifier_like",
            Self::TagList => "tag_list",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ExtractionFilterValue {
    String(String),
    Strings(Vec<String>),
    Integers(Vec<i64>),
    Bool(bool),
    Integer(i64),
}

impl ExtractionFilterValue {
    fn as_string(&self) -> String {
        match self {
            Self::String(value) => value.clone(),
            Self::Strings(values) => values.join(","),
            Self::Integers(values) => values
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            Self::Bool(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
        }
    }

    fn strings(&self) -> Vec<String> {
        match self {
            Self::Strings(values) => values.clone(),
            Self::Integers(values) => values.iter().map(i64::to_string).collect(),
            value => vec![value.as_string()],
        }
    }

    fn integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::String(value) => parse_i64(value),
            Self::Strings(_) | Self::Integers(_) | Self::Bool(_) => None,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, Self::Strings(_) | Self::Integers(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionFilterInput {
    kind: String,
    asset_path: String,
    text: String,
    context: BTreeMap<String, String>,
}

impl ExtractionFilterInput {
    pub fn new(
        kind: impl Into<String>,
        asset_path: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            asset_path: asset_path.into(),
            text: text.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    pub fn insert_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context.insert(key.into(), value.into());
    }

    fn field_value(&self, field: &str) -> Option<&str> {
        match field {
            "kind" => Some(&self.kind),
            "asset_path" => Some(&self.asset_path),
            "path" => self
                .context
                .get("path")
                .map(String::as_str)
                .or(Some(&self.asset_path)),
            "text" => Some(&self.text),
            key => self.context.get(key).map(String::as_str),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionFilterMatch {
    rule_id: String,
    reason: String,
}

impl ExtractionFilterMatch {
    pub fn new(rule_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            reason: reason.into(),
        }
    }

    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionFilterSet {
    rules: Vec<CompiledRule>,
    config: Option<ExtractionFilterConfig>,
}

impl Default for ExtractionFilterSet {
    fn default() -> Self {
        Self {
            rules: builtin_rules(),
            config: None,
        }
    }
}

impl ExtractionFilterSet {
    pub fn from_config(config: ExtractionFilterConfig) -> Result<Self, ExtractionFilterError> {
        let mut rules = builtin_rules();
        let mut seen = BTreeSet::new();
        for item in &config.rules {
            validate_rule_id(&item.id)?;
            if !seen.insert(item.id.clone()) {
                return Err(ExtractionFilterError::DuplicateRule {
                    id: item.id.clone(),
                });
            }
            let position = rules.iter().position(|rule| rule.id == item.id);
            let expression = match &item.when {
                Some(expression) => Some(compile_expression(&item.id, expression.clone())?),
                None => None,
            };
            if let Some(index) = position {
                rules[index].enabled = item.enabled;
                if let Some(reason) = &item.reason {
                    rules[index].reason = reason.clone();
                }
                if let Some(expression) = expression {
                    rules[index].expression = expression;
                }
                continue;
            }
            let expression =
                expression.ok_or_else(|| ExtractionFilterError::MissingExpression {
                    id: item.id.clone(),
                })?;
            rules.push(CompiledRule {
                id: item.id.clone(),
                enabled: item.enabled,
                reason: item.reason.clone().unwrap_or(item.id.clone()),
                expression,
            });
        }
        Ok(Self {
            rules,
            config: Some(config),
        })
    }

    pub fn config(&self) -> Option<&ExtractionFilterConfig> {
        self.config.as_ref()
    }

    pub fn is_default(&self) -> bool {
        self.config.is_none()
    }

    pub fn evaluate(&self, input: &ExtractionFilterInput) -> Option<ExtractionFilterMatch> {
        self.rules
            .iter()
            .filter(|rule| rule.enabled)
            .find(|rule| rule.expression.matches(input))
            .map(|rule| ExtractionFilterMatch::new(rule.id.clone(), rule.reason.clone()))
    }
}

pub fn evaluate_builtin(kind: &str, text: &str) -> Option<ExtractionFilterMatch> {
    if text.trim().is_empty() {
        return Some(ExtractionFilterMatch::new(
            "core.empty_source",
            "empty source",
        ));
    }
    if kind == "pex" && identifier_like_source(text) {
        return Some(ExtractionFilterMatch::new(
            "pex.identifier_like_source",
            "identifier-like source",
        ));
    }
    if kind == "pex" && tag_list_source(text) {
        return Some(ExtractionFilterMatch::new(
            "pex.tag_list_source",
            "tag-list source",
        ));
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledRule {
    id: String,
    enabled: bool,
    reason: String,
    expression: CompiledExpression,
}

#[derive(Debug, Clone)]
enum CompiledExpression {
    All(Vec<CompiledExpression>),
    Any(Vec<CompiledExpression>),
    Not(Box<CompiledExpression>),
    Condition(CompiledCondition),
}

impl PartialEq for CompiledExpression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::All(left), Self::All(right)) | (Self::Any(left), Self::Any(right)) => {
                left == right
            }
            (Self::Not(left), Self::Not(right)) => left == right,
            (Self::Condition(left), Self::Condition(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for CompiledExpression {}

impl CompiledExpression {
    fn matches(&self, input: &ExtractionFilterInput) -> bool {
        match self {
            Self::All(items) => items.iter().all(|item| item.matches(input)),
            Self::Any(items) => items.iter().any(|item| item.matches(input)),
            Self::Not(item) => !item.matches(input),
            Self::Condition(condition) => condition.matches(input),
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledCondition {
    field: String,
    op: ExtractionFilterOperator,
    value: Option<ExtractionFilterValue>,
    regex: Option<Regex>,
}

impl PartialEq for CompiledCondition {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.op == other.op && self.value == other.value
    }
}

impl Eq for CompiledCondition {}

impl CompiledCondition {
    fn matches(&self, input: &ExtractionFilterInput) -> bool {
        let field = input.field_value(&self.field);
        match self.op {
            ExtractionFilterOperator::Exists => field.is_some(),
            ExtractionFilterOperator::IsEmpty => field.is_some_and(|value| value.trim().is_empty()),
            ExtractionFilterOperator::IdentifierLike => field.is_some_and(identifier_like_source),
            ExtractionFilterOperator::TagList => field.is_some_and(tag_list_source),
            ExtractionFilterOperator::Eq => self.value.as_ref().is_some_and(|value| {
                field.is_some_and(|field| values_equal(&self.field, field, value))
            }),
            ExtractionFilterOperator::Ne => self.value.as_ref().is_some_and(|value| {
                field.is_some_and(|field| !values_equal(&self.field, field, value))
            }),
            ExtractionFilterOperator::In => self.value.as_ref().is_some_and(|value| {
                field.is_some_and(|field| {
                    value
                        .strings()
                        .iter()
                        .any(|item| value_text_equal(&self.field, field, item))
                })
            }),
            ExtractionFilterOperator::Contains => self
                .value
                .as_ref()
                .is_some_and(|value| field.is_some_and(|field| field.contains(&value.as_string()))),
            ExtractionFilterOperator::StartsWith => self.value.as_ref().is_some_and(|value| {
                field.is_some_and(|field| field.starts_with(&value.as_string()))
            }),
            ExtractionFilterOperator::EndsWith => self.value.as_ref().is_some_and(|value| {
                field.is_some_and(|field| field.ends_with(&value.as_string()))
            }),
            ExtractionFilterOperator::Regex => self
                .regex
                .as_ref()
                .is_some_and(|regex| field.is_some_and(|field| regex.is_match(field))),
        }
    }
}

fn builtin_rules() -> Vec<CompiledRule> {
    vec![
        CompiledRule {
            id: "core.empty_source".to_string(),
            enabled: true,
            reason: "empty source".to_string(),
            expression: CompiledExpression::Condition(CompiledCondition {
                field: "text".to_string(),
                op: ExtractionFilterOperator::IsEmpty,
                value: None,
                regex: None,
            }),
        },
        CompiledRule {
            id: "pex.identifier_like_source".to_string(),
            enabled: true,
            reason: "identifier-like source".to_string(),
            expression: CompiledExpression::All(vec![
                CompiledExpression::Condition(CompiledCondition {
                    field: "kind".to_string(),
                    op: ExtractionFilterOperator::Eq,
                    value: Some(ExtractionFilterValue::String("pex".to_string())),
                    regex: None,
                }),
                CompiledExpression::Condition(CompiledCondition {
                    field: "text".to_string(),
                    op: ExtractionFilterOperator::IdentifierLike,
                    value: None,
                    regex: None,
                }),
            ]),
        },
        CompiledRule {
            id: "pex.tag_list_source".to_string(),
            enabled: true,
            reason: "tag-list source".to_string(),
            expression: CompiledExpression::All(vec![
                CompiledExpression::Condition(CompiledCondition {
                    field: "kind".to_string(),
                    op: ExtractionFilterOperator::Eq,
                    value: Some(ExtractionFilterValue::String("pex".to_string())),
                    regex: None,
                }),
                CompiledExpression::Condition(CompiledCondition {
                    field: "text".to_string(),
                    op: ExtractionFilterOperator::TagList,
                    value: None,
                    regex: None,
                }),
            ]),
        },
    ]
}

fn compile_expression(
    id: &str,
    expression: ExtractionFilterExpression,
) -> Result<CompiledExpression, ExtractionFilterError> {
    Ok(match expression {
        ExtractionFilterExpression::All { all } => {
            if all.is_empty() {
                return Err(ExtractionFilterError::EmptyGroup {
                    id: id.to_string(),
                    group: "all",
                });
            }
            CompiledExpression::All(
                all.into_iter()
                    .map(|item| compile_expression(id, item))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        ExtractionFilterExpression::Any { any } => {
            if any.is_empty() {
                return Err(ExtractionFilterError::EmptyGroup {
                    id: id.to_string(),
                    group: "any",
                });
            }
            CompiledExpression::Any(
                any.into_iter()
                    .map(|item| compile_expression(id, item))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        ExtractionFilterExpression::Not { not } => {
            CompiledExpression::Not(Box::new(compile_expression(id, *not)?))
        }
        ExtractionFilterExpression::Condition { field, op, value } => {
            validate_field(id, &field)?;
            let op = parse_operator(id, &op)?;
            validate_value(id, op, value.as_ref())?;
            let regex = if op == ExtractionFilterOperator::Regex {
                let pattern = value
                    .as_ref()
                    .expect("regex value was validated")
                    .as_string();
                Some(RegexBuilder::new(&pattern).build().map_err(|source| {
                    ExtractionFilterError::InvalidRegex {
                        id: id.to_string(),
                        message: source.to_string(),
                    }
                })?)
            } else {
                None
            };
            CompiledExpression::Condition(CompiledCondition {
                field,
                op,
                value,
                regex,
            })
        }
    })
}

fn validate_rule_id(id: &str) -> Result<(), ExtractionFilterError> {
    let valid_chars = id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'));
    if id.is_empty()
        || !id.contains('.')
        || !valid_chars
        || !(BUILTIN_RULE_IDS.contains(&id) || id.starts_with("user."))
    {
        return Err(ExtractionFilterError::InvalidRuleId { id: id.to_string() });
    }
    Ok(())
}

fn validate_field(id: &str, field: &str) -> Result<(), ExtractionFilterError> {
    let valid = !field.is_empty()
        && field
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_'));
    if !valid {
        return Err(ExtractionFilterError::InvalidField {
            id: id.to_string(),
            field: field.to_string(),
        });
    }
    Ok(())
}

fn parse_operator(
    id: &str,
    operator: &str,
) -> Result<ExtractionFilterOperator, ExtractionFilterError> {
    match operator {
        "eq" => Ok(ExtractionFilterOperator::Eq),
        "ne" => Ok(ExtractionFilterOperator::Ne),
        "in" => Ok(ExtractionFilterOperator::In),
        "contains" => Ok(ExtractionFilterOperator::Contains),
        "starts_with" => Ok(ExtractionFilterOperator::StartsWith),
        "ends_with" => Ok(ExtractionFilterOperator::EndsWith),
        "regex" => Ok(ExtractionFilterOperator::Regex),
        "exists" => Ok(ExtractionFilterOperator::Exists),
        "is_empty" => Ok(ExtractionFilterOperator::IsEmpty),
        "identifier_like" => Ok(ExtractionFilterOperator::IdentifierLike),
        "tag_list" => Ok(ExtractionFilterOperator::TagList),
        value => Err(ExtractionFilterError::InvalidOperator {
            id: id.to_string(),
            operator: value.to_string(),
        }),
    }
}

fn validate_value(
    id: &str,
    op: ExtractionFilterOperator,
    value: Option<&ExtractionFilterValue>,
) -> Result<(), ExtractionFilterError> {
    let requires_value = matches!(
        op,
        ExtractionFilterOperator::Eq
            | ExtractionFilterOperator::Ne
            | ExtractionFilterOperator::In
            | ExtractionFilterOperator::Contains
            | ExtractionFilterOperator::StartsWith
            | ExtractionFilterOperator::EndsWith
            | ExtractionFilterOperator::Regex
    );
    if requires_value && value.is_none() {
        return Err(ExtractionFilterError::MissingValue {
            id: id.to_string(),
            operator: op.name().to_string(),
        });
    }
    if !requires_value && value.is_some() {
        return Err(ExtractionFilterError::UnexpectedValue {
            id: id.to_string(),
            operator: op.name().to_string(),
        });
    }
    if let Some(value) = value {
        if op == ExtractionFilterOperator::In && !value.is_list() {
            return Err(ExtractionFilterError::InvalidValue {
                id: id.to_string(),
                operator: op.name().to_string(),
                expected: "a list",
            });
        }
        if op != ExtractionFilterOperator::In && value.is_list() {
            return Err(ExtractionFilterError::InvalidValue {
                id: id.to_string(),
                operator: op.name().to_string(),
                expected: "a scalar",
            });
        }
    }
    Ok(())
}

fn values_equal(field_name: &str, field: &str, value: &ExtractionFilterValue) -> bool {
    value_text_equal(field_name, field, &value.as_string())
        || value
            .integer()
            .zip(parse_numeric_field(field_name, field))
            .is_some_and(|(left, right)| left == right)
}

fn value_text_equal(field_name: &str, field: &str, value: &str) -> bool {
    field == value
        || parse_numeric_field(field_name, field)
            .zip(parse_i64(value))
            .is_some_and(|(left, right)| left == right)
}

fn parse_numeric_field(field_name: &str, value: &str) -> Option<i64> {
    if !matches!(field_name, "form_id" | "string_id") {
        return None;
    }
    parse_i64(value)
}

fn parse_i64(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return i64::from_str_radix(hex, 16).ok();
    }
    trimmed.parse().ok()
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
    if !trimmed.contains(',') {
        return false;
    }
    let mut count = 0;
    for token in trimmed.split(',') {
        let token = token.trim();
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

fn default_enabled() -> bool {
    true
}
