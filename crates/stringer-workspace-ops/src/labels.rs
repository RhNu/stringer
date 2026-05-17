use std::collections::BTreeMap;

pub fn workspace_context_label(file: &str, context: &BTreeMap<String, String>) -> String {
    if file.starts_with("entries/plugin/") {
        return label_from_keys("plugin", context, &["record_type", "subrecord", "form_id"]);
    }
    if file.starts_with("entries/pex/") {
        return label_from_keys(
            "pex",
            context,
            &["object", "state", "function", "opcode", "operand"],
        );
    }
    if file.starts_with("entries/scaleform/") {
        return label_from_keys("scaleform", context, &["key"]);
    }
    label_from_keys("entry", context, &["record_type", "subrecord", "key"])
}

fn label_from_keys(prefix: &str, context: &BTreeMap<String, String>, keys: &[&str]) -> String {
    let parts = keys
        .iter()
        .filter_map(|key| context.get(*key))
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if parts.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {}", parts.join(" "))
    }
}
