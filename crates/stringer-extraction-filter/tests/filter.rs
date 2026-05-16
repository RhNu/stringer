use stringer_extraction_filter::{
    ExtractionFilterConfig, ExtractionFilterInput, ExtractionFilterSet,
};

fn input(text: &str) -> ExtractionFilterInput {
    ExtractionFilterInput::new("pex", "Scripts/Example.pex", text)
        .with_context("function", "Run")
        .with_context("call_member", "Notification")
}

fn scoped_input(kind: &str, text: &str) -> ExtractionFilterInput {
    ExtractionFilterInput::new(kind, "Scripts/Example.pex", text)
        .with_context("function", "Run")
        .with_context("call_member", "Notification")
}

#[test]
fn expression_tree_supports_all_any_and_not() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.debug_message"
when = { all = [
  { field = "kind", op = "eq", value = "pex" },
  { any = [
    { field = "text", op = "contains", value = "DEBUG" },
    { field = "text", op = "contains", value = "TODO" },
  ] },
  { not = { field = "function", op = "eq", value = "TranslateMe" } },
] }
"#,
    )
    .unwrap();
    let filter = ExtractionFilterSet::from_config(config).unwrap();

    assert_eq!(
        filter.evaluate(&input("DEBUG: opened")).unwrap().rule_id(),
        "user.debug_message"
    );
    assert!(filter.evaluate(&input("Open Door")).is_none());
    assert!(
        filter
            .evaluate(
                &ExtractionFilterInput::new("pex", "Scripts/Example.pex", "DEBUG: opened")
                    .with_context("function", "TranslateMe")
            )
            .is_none()
    );
}

#[test]
fn regex_rules_report_invalid_patterns() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.bad_regex"
when = { field = "text", op = "regex", value = "[" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid extraction filter regex")
    );
    assert!(error.to_string().contains("user.bad_regex"));
}

#[test]
fn invalid_operators_are_reported_as_filter_errors() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.bad_operator"
when = { field = "text", op = "matches", value = "x" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid extraction filter operator")
    );
    assert!(error.to_string().contains("matches"));
}

#[test]
fn unknown_fields_are_rejected() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.bad_field"
when = { field = "tex", op = "contains", value = "x" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid extraction filter field")
    );
    assert!(error.to_string().contains("tex"));
}

#[test]
fn empty_logical_groups_are_rejected() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.empty_group"
when = { all = [] }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(error.to_string().contains("empty extraction filter group"));
}

#[test]
fn extra_expression_keys_are_rejected() {
    let error = toml::from_str::<ExtractionFilterConfig>(
        r#"
[[rules]]
id = "user.extra_key"
when = { all = [
  { field = "text", op = "contains", value = "DEBUG" }
], field = "text" }
"#,
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("unknown field") || message.contains("did not match any variant"));
}

#[test]
fn valueless_operators_reject_values() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.unexpected_value"
when = { field = "text", op = "is_empty", value = "ignored" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(error.to_string().contains("must not define `value`"));
}

#[test]
fn scalar_operators_reject_list_values() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.array_eq"
when = { field = "text", op = "eq", value = ["a", "b"] }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(error.to_string().contains("expected a scalar"));
}

#[test]
fn in_operator_rejects_scalar_values() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.scalar_in"
when = { field = "text", op = "in", value = "a" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(error.to_string().contains("expected a list"));
}

#[test]
fn numeric_values_match_hex_context_fields() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.form"
when = { field = "form_id", op = "eq", value = 0x800 }
"#,
    )
    .unwrap();
    let filter = ExtractionFilterSet::from_config(config).unwrap();

    let matched = filter
        .evaluate(
            &ExtractionFilterInput::new("plugin", "MyMod.esp", "Iron Sword")
                .with_context("form_id", "0x00000800"),
        )
        .unwrap();

    assert_eq!(matched.rule_id(), "user.form");
}

#[test]
fn numeric_equivalence_does_not_apply_to_text_fields() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.text_number"
when = { field = "text", op = "eq", value = "1" }
"#,
    )
    .unwrap();
    let filter = ExtractionFilterSet::from_config(config).unwrap();

    assert!(filter.evaluate(&input("01")).is_none());
    assert!(filter.evaluate(&input("0x1")).is_none());
    assert_eq!(
        filter.evaluate(&input("1")).unwrap().rule_id(),
        "user.text_number"
    );
}

#[test]
fn numeric_lists_match_hex_context_fields() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.form_list"
when = { field = "form_id", op = "in", value = [0x800, 0x801] }
"#,
    )
    .unwrap();
    let filter = ExtractionFilterSet::from_config(config).unwrap();

    let matched = filter
        .evaluate(
            &ExtractionFilterInput::new("plugin", "MyMod.esp", "Iron Sword")
                .with_context("form_id", "0x00000801"),
        )
        .unwrap();

    assert_eq!(matched.rule_id(), "user.form_list");
}

#[test]
fn builtin_rules_can_be_disabled_by_id() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "pex.identifier_like_source"
enabled = false
"#,
    )
    .unwrap();
    let filter = ExtractionFilterSet::from_config(config).unwrap();

    assert!(filter.evaluate(&input("SomeIdentifier")).is_none());
    assert_eq!(
        filter.evaluate(&input("tag,tag,tag")).unwrap().rule_id(),
        "pex.tag_list_source"
    );
}

#[test]
fn duplicate_custom_rule_ids_are_rejected() {
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.duplicate"
when = { field = "text", op = "contains", value = "one" }

[[rules]]
id = "user.duplicate"
when = { field = "text", op = "contains", value = "two" }
"#,
    )
    .unwrap();

    let error = ExtractionFilterSet::from_config(config).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("duplicate extraction filter rule")
    );
}

#[test]
fn identifier_like_and_tag_list_match_existing_pex_behavior() {
    let filter = ExtractionFilterSet::default();

    assert_eq!(
        filter
            .evaluate(&input("Namespace.Member"))
            .unwrap()
            .rule_id(),
        "pex.identifier_like_source"
    );
    assert_eq!(
        filter.evaluate(&input("foo_bar,baz-1")).unwrap().rule_id(),
        "pex.tag_list_source"
    );
    assert!(filter.evaluate(&input("Open Door")).is_none());
}

#[test]
fn builtin_tag_list_filters_comma_space_pex_sources_only() {
    let filter = ExtractionFilterSet::default();

    assert_eq!(
        filter
            .evaluate(&scoped_input("pex", "MF, Tag, U"))
            .unwrap()
            .rule_id(),
        "pex.tag_list_source"
    );
    assert!(
        filter
            .evaluate(&scoped_input("plugin", "MF, Tag, U"))
            .is_none()
    );
}
