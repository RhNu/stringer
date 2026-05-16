pub(crate) const ROOT_LONG_ABOUT: &str = r#"Stringer opens Bethesda mod text into an agent-editable translation workspace, enriches it with knowledge, and finalizes translated assets into an output directory."#;

pub(crate) const ROOT_AFTER_LONG_HELP: &str = r#"Typical workflow:
  stringer workspace open --source-root <SOURCE_ROOT> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
  stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --source-locale en --target-locale zh-Hans --game SkyrimSe
  stringer knowledge annotate
  stringer workspace batch count --json
  stringer workspace batch claim --limit 50
  stringer workspace batch read --batch-id <BATCH_ID> --limit 10
  stringer workspace batch submit --input <PATCH_JSON>
  stringer knowledge validate
  stringer workspace finalize

Workspace layout: workspace.json, batches/, entries/**/*.jsonl.
Knowledge layers: user global knowledge, then workspace knowledge/{terms,memory,rules}; workspace ids override global ids. Derived indexes stay per layer.
Feedback: interactive runs show progress on stderr; use --progress always, --progress never, --quiet, -v for default tracing, or RUST_LOG for explicit trace filters.
See README.md and skills/stringer-workflows for agent workflows."#;

pub(crate) const SETTINGS_LONG_HELP: &str =
    r#"Use explicit settings for reproducible agent runs, or configure them in stringer.toml."#;

pub(crate) const WORKSPACE_LONG_ABOUT: &str =
    r#"Manage translation workspace open, batch, finalize, and upgrade commands."#;

pub(crate) const WORKSPACE_OPEN_LONG_ABOUT: &str = r#"Scan a read-only source root and write an editable workspace with workspace.json, batches/, and entries/**/*.jsonl."#;

pub(crate) const WORKSPACE_OPEN_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace open --source-root ../MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

Common next step:
  stringer knowledge annotate

The workspace defaults to the current directory. Source root settings are not read; use stringer.toml in the workspace or pass settings on the command line."#;

pub(crate) const WORKSPACE_FINALIZE_LONG_ABOUT: &str = r#"Apply translated workspace rows to the stored source root and write changed files to an output directory."#;

pub(crate) const WORKSPACE_FINALIZE_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace finalize --output ./output

Run knowledge validate first. The output defaults to <workspace>/output and must stay outside the source root. Use --source-root only to override the source_root stored in workspace.json."#;

pub(crate) const WORKSPACE_BATCH_LONG_ABOUT: &str =
    r#"Count, claim, read, submit, export, and release translation batches for agent work."#;

pub(crate) const WORKSPACE_BATCH_COUNT_LONG_ABOUT: &str =
    r#"Count total, empty, memory-prefilled, translated, skipped, claimed, and diagnostic rows."#;

pub(crate) const WORKSPACE_BATCH_CLAIM_LONG_ABOUT: &str = r#"Claim eligible untranslated or memory-prefilled rows and print a compact batch summary. Read claimed entries with workspace batch read."#;

pub(crate) const WORKSPACE_BATCH_READ_LONG_ABOUT: &str = r#"Read compact source, current translation, context label, hint counts, and diagnostic codes for a claimed batch."#;

pub(crate) const WORKSPACE_BATCH_DETAIL_LONG_ABOUT: &str =
    r#"Read full context, hints, diagnostics, and metadata for one or more claimed batch keys."#;

pub(crate) const WORKSPACE_BATCH_SUBMIT_LONG_ABOUT: &str =
    r#"Submit translate, skip, or pending actions for batch-local keys from JSON or CSV."#;

pub(crate) const WORKSPACE_BATCH_EXPORT_LONG_ABOUT: &str = r#"Export a claimed batch to an editable JSON or CSV submission file under batch-work by default."#;

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
  stringer knowledge index rebuild"#;

pub(crate) const KNOWLEDGE_LONG_ABOUT: &str =
    r#"Annotate, validate, lookup, and index terminology, memory, rules, and diagnostics."#;

pub(crate) const ANNOTATE_LONG_ABOUT: &str = r#"Write terminology and memory hints into a workspace, optionally filling high-confidence memory translations."#;

pub(crate) const ANNOTATE_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge annotate
  stringer knowledge annotate --workspace ./translations --skip-fill-memory

Progress is shown on stderr for interactive runs; stdout remains a final summary line."#;

pub(crate) const VALIDATE_LONG_ABOUT: &str =
    r#"Recompute workspace diagnostics before review or finalize."#;

pub(crate) const VALIDATE_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge validate

Common diagnostics: term.preferred_missing, term.forbidden_used, placeholder.mismatch, scaleform.newline, translation.empty, memory.conflict."#;

pub(crate) const LOOKUP_LONG_ABOUT: &str =
    r#"Search terminology and translation memory for agent-readable evidence."#;

pub(crate) const LOOKUP_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge lookup --text "Altmer" --kind plugin --record-type NPC_ --json
  stringer knowledge lookup --text "^(Alt|Bos)mer$" --regex --source memory --field source --json

Defaults: --source all --field both --limit 20."#;

pub(crate) const INDEX_LONG_ABOUT: &str = r#"Maintain the derived knowledge SQLite index."#;

pub(crate) const INDEX_REBUILD_LONG_ABOUT: &str =
    r#"Rebuild global and workspace knowledge SQLite indexes from knowledge files."#;

pub(crate) const INDEX_REBUILD_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge index rebuild"#;
