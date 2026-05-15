pub(crate) const ROOT_LONG_ABOUT: &str = r#"Stringer opens Bethesda mod text into an agent-editable translation workspace, enriches it with knowledge, and finalizes translated assets into an override directory."#;

pub(crate) const ROOT_AFTER_LONG_HELP: &str = r#"Typical workflow:
  stringer workspace open --root <MOD_ROOT> --workspace <WORKSPACE> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
  stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --source-locale en --target-locale zh-Hans --game SkyrimSe
  stringer knowledge annotate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
  stringer workspace batch count --workspace <WORKSPACE> --json
  stringer workspace batch claim --workspace <WORKSPACE> --limit 50
  stringer workspace batch apply --workspace <WORKSPACE> --input <PATCH_JSON>
  stringer knowledge validate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
  stringer workspace finalize --root <MOD_ROOT> --workspace <WORKSPACE> --override-root <OVERRIDE_ROOT>

Workspace layout: workspace.json, batches/, entries/**/*.jsonl.
Knowledge: <PROJECT_ROOT>/knowledge/{terms,memory,rules}.
See README.md and skills/stringer-workflows for agent workflows."#;

pub(crate) const SETTINGS_LONG_HELP: &str =
    r#"Use explicit settings for reproducible agent runs, or configure them in stringer.toml."#;

pub(crate) const WORKSPACE_LONG_ABOUT: &str =
    r#"Manage translation workspace open, batch, finalize, and upgrade commands."#;

pub(crate) const WORKSPACE_OPEN_LONG_ABOUT: &str = r#"Scan a mod root and write an editable workspace with workspace.json, batches/, and entries/**/*.jsonl."#;

pub(crate) const WORKSPACE_OPEN_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace open --root ./MyMod --workspace ./translations --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

Common next step:
  stringer knowledge annotate --project-root ./MyMod --workspace ./translations"#;

pub(crate) const WORKSPACE_FINALIZE_LONG_ABOUT: &str = r#"Apply translated workspace rows to the source assets and write changed files to an override directory."#;

pub(crate) const WORKSPACE_FINALIZE_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace finalize --root ./MyMod --workspace ./translations --override-root ./StringerOverride

Run knowledge validate first. Use an override directory outside the source mod root."#;

pub(crate) const WORKSPACE_BATCH_LONG_ABOUT: &str =
    r#"Count, claim, apply, and release translation batches for agent work."#;

pub(crate) const WORKSPACE_BATCH_COUNT_LONG_ABOUT: &str =
    r#"Count total, empty, memory-prefilled, translated, claimed, and diagnostic rows."#;

pub(crate) const WORKSPACE_BATCH_CLAIM_LONG_ABOUT: &str = r#"Claim eligible untranslated or memory-prefilled rows and print source, context, hints, and diagnostics as JSON."#;

pub(crate) const WORKSPACE_BATCH_APPLY_LONG_ABOUT: &str =
    r#"Apply translations for ids owned by a claimed batch."#;

pub(crate) const WORKSPACE_BATCH_RELEASE_LONG_ABOUT: &str =
    r#"Release a claimed batch so its remaining entries can be claimed again."#;

pub(crate) const WORKSPACE_UPGRADE_LONG_ABOUT: &str =
    r#"Report that legacy manifest.json workspace upgrades are not implemented yet."#;

pub(crate) const WORKSPACE_UPGRADE_AFTER_LONG_HELP: &str =
    r#"Recreate legacy workspaces with workspace open until upgrade support exists."#;

pub(crate) const ADAPT_LONG_ABOUT: &str =
    r#"Convert external translation resources into Stringer translation memory."#;

pub(crate) const ADAPT_IMPORT_LONG_ABOUT: &str =
    r#"Import EET, EET XML, EET JSON, or xTranslator SST resources as translation memory JSONL."#;

pub(crate) const ADAPT_IMPORT_AFTER_LONG_HELP: &str = r#"Example:
  stringer adapt import --format xt-sst --input ./old.sst --source-locale en --target-locale zh-Hans --game SkyrimSe

Common next step:
  stringer knowledge index rebuild --project-root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;

pub(crate) const KNOWLEDGE_LONG_ABOUT: &str =
    r#"Annotate, validate, lookup, and index terminology, memory, rules, and diagnostics."#;

pub(crate) const ANNOTATE_LONG_ABOUT: &str = r#"Write terminology and memory hints into a workspace, optionally filling high-confidence memory translations."#;

pub(crate) const ANNOTATE_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge annotate --project-root ./MyMod --workspace ./translations
  stringer knowledge annotate --workspace ./translations --skip-fill-memory"#;

pub(crate) const VALIDATE_LONG_ABOUT: &str =
    r#"Recompute workspace diagnostics before review or finalize."#;

pub(crate) const VALIDATE_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge validate --project-root ./MyMod --workspace ./translations

Common diagnostics: term.preferred_missing, term.forbidden_used, placeholder.mismatch, scaleform.newline, translation.empty, memory.conflict."#;

pub(crate) const LOOKUP_LONG_ABOUT: &str =
    r#"Search terminology and translation memory for agent-readable evidence."#;

pub(crate) const LOOKUP_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge lookup --project-root ./MyMod --text "Altmer" --kind plugin --record-type NPC_ --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans --json
  stringer knowledge lookup --text "^(Alt|Bos)mer$" --regex --source memory --field source --json

Defaults: --source all --field both --limit 20."#;

pub(crate) const INDEX_LONG_ABOUT: &str = r#"Maintain the derived knowledge SQLite index."#;

pub(crate) const INDEX_REBUILD_LONG_ABOUT: &str =
    r#"Rebuild <PROJECT_ROOT>/.stringer/indexes/knowledge.sqlite from knowledge files."#;

pub(crate) const INDEX_REBUILD_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge index rebuild --project-root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;
