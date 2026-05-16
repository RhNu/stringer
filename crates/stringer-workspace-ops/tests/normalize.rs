use stringer_workspace_ops::{
    NormalizeRuleEncoding, NormalizeWorkspaceOptions, WorkspaceOpsError, normalize_workspace,
};

mod support;

use support::*;

#[test]
fn normalize_dry_run_decodes_cp936_rules_and_does_not_write() {
    let fixture = workspace_with_rows(
        "normalize-dry-run",
        r#"{"id":"scaleform:MyMod:$Desc","source":"Steel Sword","translation":"钢剑","translation_meta":{"origin":"memory"}}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_bytes(&rules, &cp936_rule("钢剑", "熟铁剑"));

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: Some(ENTRY_FILE.to_string()),
        apply: false,
        encoding: NormalizeRuleEncoding::Auto,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.scanned_entries, 1);
    assert_eq!(summary.changed_entries, 1);
    assert_eq!(summary.total_replacements, 1);
    assert_eq!(summary.changes.len(), 1);
    assert_eq!(summary.changes[0].before, "钢剑");
    assert_eq!(summary.changes[0].after, "熟铁剑");

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "钢剑");
    assert_eq!(rows[0]["translation_meta"]["origin"], "memory");
}

#[test]
fn normalize_apply_writes_safe_changes_and_preserves_translation_origin() {
    let fixture = workspace_with_rows(
        "normalize-apply",
        concat!(
            "{\"id\":\"scaleform:MyMod:$Desc\",\"source\":\"Steel Sword\",\"translation\":\"钢剑\",\"translation_meta\":{\"origin\":\"memory\"}}\n",
            "{\"id\":\"scaleform:MyMod:$Done\",\"source\":\"Done\",\"translation\":\"魔族剑\",\"translation_meta\":{\"origin\":\"agent\"}}\n",
        ),
    );
    write_batch(fixture.workspace(), "b-active", &["scaleform:MyMod:$Done"]);
    let rules = fixture.workspace().join("rules.txt");
    write_text(
        &rules,
        concat!(
            "StartRule\nSearch=钢剑\nReplace=熟铁剑\nmode=0\nselect=0\nAllLists=1\nEndRule\n",
            "StartRule\nSearch=魔族剑\nReplace=魔族\nEndRule\n",
            "StartRule\nSearch=魔族\nReplace=魔人\nEndRule\n",
        ),
    );

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: Some(ENTRY_FILE.to_string()),
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.scanned_entries, 1);
    assert_eq!(summary.changed_entries, 1);
    assert_eq!(summary.total_replacements, 1);
    assert_eq!(summary.skipped_claimed, 1);

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "熟铁剑");
    assert_eq!(rows[0]["translation_meta"]["origin"], "memory");
    assert!(rows[0]["translation_meta"]["updated_at_unix_ms"].is_number());
    assert_eq!(rows[1]["translation"], "魔族剑");
    assert_eq!(rows[1]["translation_meta"]["origin"], "agent");
}

#[test]
fn normalize_applies_rules_in_file_order_for_cascading_replacements() {
    let fixture = workspace_with_rows(
        "normalize-cascade",
        r#"{"id":"scaleform:MyMod:$Done","source":"Done","translation":"魔族剑"}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(
        &rules,
        concat!(
            "StartRule\nSearch=魔族剑\nReplace=魔族\nEndRule\n",
            "StartRule\nSearch=魔族\nReplace=魔人\nEndRule\n",
        ),
    );

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 1);
    assert_eq!(summary.total_replacements, 2);

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "魔人");
    assert_eq!(rows[0]["translation_meta"]["origin"], "normalized");
}

#[test]
fn normalize_skips_placeholder_risk_without_writing() {
    let fixture = workspace_with_rows(
        "normalize-placeholder-risk",
        r#"{"id":"scaleform:MyMod:$Alias","source":"Alias","translation":"<Alias=Player>魔族","translation_meta":{"origin":"agent"}}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(
        &rules,
        "StartRule\nSearch=<Alias=Player>\nReplace=玩家\nEndRule\n",
    );

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 0);
    assert_eq!(summary.total_replacements, 0);
    assert_eq!(summary.skipped_placeholder_risk, 1);
    assert_eq!(summary.changes.len(), 1);
    assert!(summary.changes[0].skipped_placeholder_risk);

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "<Alias=Player>魔族");
}

#[test]
fn normalize_skips_printf_placeholder_risk_without_writing() {
    let fixture = workspace_with_rows(
        "normalize-printf-placeholder-risk",
        r#"{"id":"scaleform:MyMod:$Format","source":"Format","translation":"你好 %s","translation_meta":{"origin":"agent"}}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(&rules, "StartRule\nSearch=%s\nReplace=玩家\nEndRule\n");

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 0);
    assert_eq!(summary.total_replacements, 0);
    assert_eq!(summary.skipped_placeholder_risk, 1);
    assert!(summary.changes[0].skipped_placeholder_risk);

    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "你好 %s");
}

#[test]
fn normalize_apply_without_safe_changes_does_not_rewrite_entry_file() {
    let fixture = workspace_with_rows(
        "normalize-no-safe-changes",
        r#"{"translation_meta":{"origin":"agent"},"translation":"<Alias=Player>魔族","source":"Alias","id":"scaleform:MyMod:$Alias"}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(
        &rules,
        "StartRule\nSearch=<Alias=Player>\nReplace=玩家\nEndRule\n",
    );
    let entry_path = fixture.workspace().join(ENTRY_FILE);
    let before = std::fs::read_to_string(&entry_path).unwrap();

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 0);
    assert_eq!(std::fs::read_to_string(entry_path).unwrap(), before);
}

#[test]
fn normalize_literal_replacement_is_case_sensitive() {
    let fixture = workspace_with_rows(
        "normalize-case-sensitive",
        r#"{"id":"scaleform:MyMod:$Case","source":"Case","translation":"OR condition"}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(&rules, "StartRule\nSearch=or\nReplace=或者\nEndRule\n");

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 0);
    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "OR condition");
}

#[test]
fn normalize_decodes_utf8_bom_rules() {
    let fixture = workspace_with_rows(
        "normalize-utf8-bom",
        r#"{"id":"scaleform:MyMod:$Desc","source":"Steel Sword","translation":"钢剑"}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_bytes(
        &rules,
        "\u{feff}StartRule\nSearch=钢剑\nReplace=熟铁剑\nEndRule\n".as_bytes(),
    );

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: true,
        encoding: NormalizeRuleEncoding::Auto,
        limit: 10,
    })
    .unwrap();

    assert_eq!(summary.changed_entries, 1);
    let rows = jsonl_rows(&fixture.workspace().join(ENTRY_FILE));
    assert_eq!(rows[0]["translation"], "熟铁剑");
}

#[test]
fn normalize_reports_rule_warnings_and_rejects_unsupported_rules() {
    let fixture = workspace_with_rows(
        "normalize-warnings",
        r#"{"id":"scaleform:MyMod:$Desc","source":"Steel Sword","translation":"钢剑"}"#,
    );
    let rules = fixture.workspace().join("rules.txt");
    write_text(
        &rules,
        concat!(
            "StartRule\nSearch=钢剑\nReplace=熟铁剑\nEndRule\n",
            "StartRule\nSearch=钢剑\nReplace=铁剑\nEndRule\n",
            "StartRule\nSearch=魔族\nReplace=魔人\nEndRule\n",
        ),
    );

    let summary = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: false,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap();

    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "rule.duplicate_search_conflict")
    );
    assert!(
        summary
            .warnings
            .iter()
            .any(|warning| warning.code == "rule.short_search")
    );

    write_text(
        &rules,
        "StartRule\nSearch=钢剑\nReplace=熟铁剑\nmode=1\nEndRule\n",
    );
    let err = normalize_workspace(NormalizeWorkspaceOptions {
        workspace: utf8(fixture.workspace()),
        rules: utf8(&rules),
        file: None,
        apply: false,
        encoding: NormalizeRuleEncoding::Utf8,
        limit: 10,
    })
    .unwrap_err();
    assert!(matches!(
        err,
        WorkspaceOpsError::NormalizeRuleParse { line: 5, .. }
    ));
}

fn cp936_rule(search: &str, replace: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"StartRule\nSearch=");
    bytes.extend_from_slice(cp936_bytes(search));
    bytes.extend_from_slice(b"\nReplace=");
    bytes.extend_from_slice(cp936_bytes(replace));
    bytes.extend_from_slice(
        b"\nPattern=[%REPLACE%] %ORIG%\nselect=0\nmode=0\nAllLists=1\nEndRule\n",
    );
    bytes
}

fn cp936_bytes(value: &str) -> &'static [u8] {
    match value {
        "钢剑" => &[0xB8, 0xD6, 0xBD, 0xA3],
        "熟铁剑" => &[0xCA, 0xEC, 0xCC, 0xFA, 0xBD, 0xA3],
        _ => panic!("missing CP936 fixture bytes for {value}"),
    }
}
