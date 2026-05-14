use stringer_pipeline::{
    BasicValidationProcessor, KnowledgeBase, KnowledgeLayer, Pipeline, PipelineEntry,
    PipelineEntryKind, PipelineOptions, PipelineStage, ReplacementRuleProcessor,
    TerminologyProcessor, TranslationMemoryProcessor,
};

fn plugin_entry(source_text: &str) -> PipelineEntry {
    let mut entry = PipelineEntry::new(
        "plugin:MyMod.esp:WEAP:00001234:FULL:1",
        PipelineEntryKind::Plugin,
        source_text,
        "en",
        "zh-Hans",
        "MyMod.esp",
    );
    entry.insert_context("record_type", "WEAP");
    entry.insert_context("subrecord", "FULL");
    entry
}

fn pex_entry(source_text: &str) -> PipelineEntry {
    PipelineEntry::new(
        "pex:Scripts/Example.pex:Example::Run:0:fixed-1:1",
        PipelineEntryKind::Pex,
        source_text,
        "en",
        "zh-Hans",
        "Scripts/Example.pex",
    )
}

fn run_stage(stage: PipelineStage, entry: &mut PipelineEntry, knowledge: &KnowledgeBase) {
    let pipeline = Pipeline::new(vec![
        Box::new(TerminologyProcessor),
        Box::new(TranslationMemoryProcessor),
        Box::new(BasicValidationProcessor),
        Box::new(ReplacementRuleProcessor),
    ]);
    let report = pipeline.run_stage(
        stage,
        std::slice::from_mut(entry),
        knowledge,
        &PipelineOptions {
            allow_memory_auto_fill: true,
            execute_replacements: false,
            ..PipelineOptions::default()
        },
    );
    assert!(report.diagnostics_by_severity("error").is_empty());
}

#[test]
fn term_lookup_respects_alias_case_scope_and_layer_order() {
    let mut base = KnowledgeLayer::new("base");
    base.add_terms_toml(
        "knowledge/terms/base.toml",
        r#"
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
aliases = ["iron blade"]
case_sensitive = false
status = "preferred"
scope = { target_locale = "zh-Hans", kind = "plugin", record_type = "WEAP" }
"#,
    )
    .unwrap();
    let mut project = KnowledgeLayer::new("project");
    project
        .add_terms_toml(
            "knowledge/terms/project.toml",
            r#"
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "熟铁剑"
aliases = ["iron blade"]
status = "preferred"
scope = { target_locale = "zh-Hans", kind = "plugin", record_type = "WEAP" }
"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![base, project]).unwrap();
    let mut entry = plugin_entry("Ancient IRON BLADE");

    run_stage(PipelineStage::Annotate, &mut entry, &knowledge);

    assert!(entry.annotations().iter().any(|annotation| {
        annotation.kind() == "term"
            && annotation.id() == "skyrim.weapon.iron_sword"
            && annotation.layer() == "project"
            && annotation.payload()["target"] == "熟铁剑"
            && annotation.match_kind() == "alias"
    }));
    assert!(
        knowledge
            .merge_diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "knowledge.override")
    );
}

#[test]
fn forbidden_terms_validate_without_replacing_text() {
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_terms_toml(
            "knowledge/terms/skyrim.toml",
            r#"
[[terms]]
id = "skyrim.dragonborn.preferred"
source = "Dragonborn"
target = "龙裔"
status = "preferred"

[[terms]]
id = "skyrim.dragonborn.forbidden"
source = "Dragonborn"
target = "抓根宝"
status = "forbidden"
"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![layer]).unwrap();
    let mut entry = plugin_entry("Dragonborn");
    entry.set_translated_text("抓根宝");

    run_stage(PipelineStage::Validate, &mut entry, &knowledge);

    assert_eq!(entry.translated_text(), Some("抓根宝"));
    assert!(
        entry
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "term.forbidden_used")
    );
    assert!(
        entry
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "term.preferred_missing")
    );
}

#[test]
fn game_scoped_terms_match_when_entry_has_game_context() {
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_terms_toml(
            "knowledge/terms/skyrim.toml",
            r#"
[[terms]]
id = "skyrim.weapon.iron_sword"
source = "Iron Sword"
target = "铁剑"
status = "preferred"
scope = { game = "SkyrimSe", target_locale = "zh-Hans" }
"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![layer]).unwrap();
    let mut entry = plugin_entry("Iron Sword");
    entry.insert_context("game", "SkyrimSe");

    run_stage(PipelineStage::Annotate, &mut entry, &knowledge);

    assert!(entry.annotations().iter().any(|annotation| {
        annotation.kind() == "term" && annotation.id() == "skyrim.weapon.iron_sword"
    }));
}

#[test]
fn validate_reports_missing_translation_for_supported_entries() {
    let knowledge = KnowledgeBase::empty();
    let mut entry = plugin_entry("Iron Sword");

    run_stage(PipelineStage::Validate, &mut entry, &knowledge);

    assert!(
        entry
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == "translation.empty")
    );
}

#[test]
fn validate_skips_pex_entries() {
    let knowledge = KnowledgeBase::empty();
    let mut entry = pex_entry("{PLAYER_NAME}");
    entry.set_translated_text("");

    run_stage(PipelineStage::Validate, &mut entry, &knowledge);

    assert!(entry.diagnostics().is_empty());
}

#[test]
fn memory_exact_and_normalized_matches_auto_fill_high_confidence() {
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_memory_jsonl(
            "knowledge/memory/project.jsonl",
            r#"{"id":"tm:1","source":"Iron Sword","target":"铁剑","source_locale":"en","target_locale":"zh-Hans","context":{"kind":"plugin","record_type":"WEAP","subrecord":"FULL"},"quality":"confirmed","created_at":"2026-05-14T00:00:00Z"}
{"id":"tm:2","source":"Steel Sword","target":"钢剑","source_locale":"en","target_locale":"zh-Hans","quality":"imported","created_at":"2026-05-14T00:00:00Z"}"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![layer]).unwrap();
    let mut exact = plugin_entry("Iron Sword");
    let mut normalized = plugin_entry("  steel   sword ");

    run_stage(PipelineStage::MemoryApply, &mut exact, &knowledge);
    run_stage(PipelineStage::MemoryApply, &mut normalized, &knowledge);

    assert_eq!(exact.translated_text(), Some("铁剑"));
    assert_eq!(normalized.translated_text(), Some("钢剑"));
    assert!(exact.annotations().iter().any(|annotation| {
        annotation.kind() == "memory"
            && annotation.match_kind() == "source"
            && annotation.confidence() == 1.0
    }));
    assert!(normalized.annotations().iter().any(|annotation| {
        annotation.kind() == "memory"
            && annotation.match_kind() == "normalized_source"
            && annotation.confidence() == 0.98
    }));
}

#[test]
fn fuzzy_memory_matches_only_annotate_and_never_auto_fill() {
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_memory_jsonl(
            "knowledge/memory/project.jsonl",
            r#"{"id":"tm:1","source":"Iron Sword","target":"铁剑","source_locale":"en","target_locale":"zh-Hans","quality":"machine","created_at":"2026-05-14T00:00:00Z"}"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![layer]).unwrap();
    let mut entry = plugin_entry("Iron-Sword!");

    run_stage(PipelineStage::Annotate, &mut entry, &knowledge);
    run_stage(PipelineStage::MemoryApply, &mut entry, &knowledge);

    assert_eq!(entry.translated_text(), None);
    assert!(entry.annotations().iter().any(|annotation| {
        annotation.kind() == "memory"
            && annotation.match_kind() == "fuzzy_source"
            && annotation.confidence() < 0.95
    }));
}

#[test]
fn replacement_rules_parse_without_executing_by_default() {
    let mut layer = KnowledgeLayer::new("project");
    layer
        .add_rules_toml(
            "knowledge/rules/replacements.toml",
            r#"
[[rules]]
id = "protect.player_name"
stage = "pre_translate"
pattern = "{PLAYER_NAME}"
replacement = "__STRINGER_TOKEN_PLAYER_NAME__"
mode = "literal"
enabled = true
scope = { kind = ["plugin"] }
"#,
        )
        .unwrap();
    let knowledge = KnowledgeBase::from_layers(vec![layer]).unwrap();
    let mut entry = plugin_entry("Hello {PLAYER_NAME}");

    run_stage(PipelineStage::PreTranslate, &mut entry, &knowledge);

    assert_eq!(entry.source_text(), "Hello {PLAYER_NAME}");
    assert!(entry.annotations().iter().any(|annotation| {
        annotation.kind() == "replacement_rule"
            && annotation.id() == "protect.player_name"
            && annotation.payload()["replacement"] == "__STRINGER_TOKEN_PLAYER_NAME__"
    }));
}
