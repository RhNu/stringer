pub(crate) const ROOT_LONG_ABOUT: &str = r#"Stringer is a Bethesda mod localization command-line tool.

It opens translatable mod assets into a JSONL translation workspace, lets a human or agent edit translations, then finalizes changed assets into an override directory. Knowledge commands add terminology, translation-memory hints, diagnostics, and agent-readable knowledge search.

Recommended agent workflow:
  1. Run `stringer --help` to understand the whole flow.
  2. Run a subcommand help page, for example `stringer workspace open --help`.
  3. Prefer explicit game, language, and locale arguments or a project stringer.toml so the command does not depend on local machine defaults."#;

pub(crate) const ROOT_AFTER_LONG_HELP: &str = r#"Typical workflow:
  stringer workspace open --root <MOD_ROOT> --workspace <WORKSPACE> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
  stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --source-locale en --target-locale zh-Hans --game SkyrimSe
  stringer knowledge annotate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
  stringer workspace batch count --workspace <WORKSPACE> --json
  stringer workspace batch claim --workspace <WORKSPACE> --limit 50
  stringer workspace batch apply --workspace <WORKSPACE> --input <PATCH_JSON>
  stringer knowledge validate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
  stringer workspace finalize --root <MOD_ROOT> --workspace <WORKSPACE> --override-root <OVERRIDE_ROOT>

Default knowledge locations:
  <PROJECT_ROOT>/knowledge/terms/*.toml
  <PROJECT_ROOT>/knowledge/memory/*.jsonl
  <PROJECT_ROOT>/knowledge/rules/*.toml

Direct JSONL fallback:
  <WORKSPACE>/entries/**/*.jsonl

See README.md for project documentation."#;

pub(crate) const SETTINGS_LONG_HELP: &str = r#"These settings decide how Stringer interprets Bethesda localized assets and translation package locales.

When omitted, commands try to read the default user configuration file and, for project-aware commands, <PROJECT_ROOT>/stringer.toml. For reproducible agent runs, pass these explicitly to workspace open, lookup, and index rebuild:
  --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

Only the user configuration file may define [knowledge].global_root. Project stringer.toml files may define only game_release, asset_language, source_locale, and target_locale."#;

pub(crate) const WORKSPACE_LONG_ABOUT: &str = r#"Workspace commands manage the editable translation workspace lifecycle.

Use workspace open to generate the editable JSONL workspace from a mod root. Use workspace finalize after editing and validation to write changed assets into an override directory."#;

pub(crate) const WORKSPACE_OPEN_LONG_ABOUT: &str = r#"Scan a mod root and open an agent-editable translation workspace.

The workspace is a directory containing workspace.json, batches/, and entries/**/*.jsonl. Each JSONL row usually contains id, source, translation, translation_meta, context, hints, and diagnostics. Fresh workspaces usually leave translation empty for a human or agent to fill.

workspace open reads the default user config, <MOD_ROOT>/stringer.toml, and command-line overrides. Project settings override user settings, and command-line settings override both."#;

pub(crate) const WORKSPACE_OPEN_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace open --root ./MyMod --workspace ./translations --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

Output layout:
  <WORKSPACE>/workspace.json
  <WORKSPACE>/batches/
  <WORKSPACE>/entries/plugin/<asset>/<record_type>.jsonl
  <WORKSPACE>/entries/pex/<asset>.jsonl
  <WORKSPACE>/entries/scaleform/<asset>.jsonl

Common next step:
  stringer knowledge annotate --project-root ./MyMod --workspace ./translations
  stringer workspace batch claim --workspace ./translations --limit 50"#;

pub(crate) const WORKSPACE_FINALIZE_LONG_ABOUT: &str = r#"Read id and translation fields from a translation workspace, apply them to source mod assets, and write changed files into an override directory.

finalize ignores hints, diagnostics, and other extension fields. Rows without translation are skipped. The override directory must not be inside the source mod root, which prevents accidental input overwrites."#;

pub(crate) const WORKSPACE_FINALIZE_AFTER_LONG_HELP: &str = r#"Example:
  stringer workspace finalize --root ./MyMod --workspace ./translations --override-root ./StringerOverride

Recommended:
  Run `stringer knowledge validate` before finalize.
  Point override-root at a fresh directory, then load that directory with a mod manager."#;

pub(crate) const WORKSPACE_BATCH_LONG_ABOUT: &str = r#"Batch commands let agents translate without directly editing JSONL rows.

Use count to estimate work, claim to reserve a batch and receive source/context/hints/diagnostics as JSON, apply to submit translations for that batch, and release to abandon a batch."#;

pub(crate) const WORKSPACE_BATCH_COUNT_LONG_ABOUT: &str = r#"Count translation work in a workspace.

The count includes total rows, empty translations, high-confidence memory prefill rows, translated rows, actively claimed rows, and rows with diagnostics. When --file is supplied, it must match an entry file listed in workspace.json."#;

pub(crate) const WORKSPACE_BATCH_CLAIM_LONG_ABOUT: &str = r#"Claim unclaimed rows for agent translation.

Eligible rows have no translation, an empty translation, or translation_meta.origin=memory. Rows with translation_meta.origin=agent or manual are not claimed. The command writes batches/<batch_id>.json and prints JSON with the source, current translation, context, hints, and diagnostics."#;

pub(crate) const WORKSPACE_BATCH_APPLY_LONG_ABOUT: &str = r#"Apply translations for a claimed batch.

The input JSON contains batch_id and entries with id and translation. The command only writes ids claimed by that batch, sets translation_meta.origin=agent, and removes applied ids from the batch."#;

pub(crate) const WORKSPACE_BATCH_RELEASE_LONG_ABOUT: &str = r#"Release a claimed batch without changing translations.

This deletes batches/<batch_id>.json so the remaining entries can be claimed again."#;

pub(crate) const WORKSPACE_UPGRADE_LONG_ABOUT: &str = r#"Placeholder for future legacy workspace upgrades.

Current Workspace v3 commands do not read old manifest.json workspaces. Recreate the workspace with workspace open to get workspace.json."#;

pub(crate) const WORKSPACE_UPGRADE_AFTER_LONG_HELP: &str = r#"This command is intentionally not implemented yet.

Current behavior:
  stringer workspace upgrade --workspace ./translations
  exits with an error explaining that v2 manifest.json workspaces are not read."#;

pub(crate) const ADAPT_LONG_ABOUT: &str = r#"Adapt external translation resources into Stringer translation memory.

adapt commands do not edit mod assets or translation packages. They read external translator files, normalize usable source/target pairs, and merge Stringer memory JSONL into the configured user global knowledge root when --out is omitted."#;

pub(crate) const ADAPT_IMPORT_LONG_ABOUT: &str = r#"Import an external translation resource as Stringer translation memory JSONL.

Supported formats:
  eet       ESP-ESM Translator / EET binary table
  eet-xml   EET XML export
  eet-json  EET JSON or DDS-style export
  xt-sst    xTranslator SST file

The output rows contain id, source, target, source_locale, target_locale, context, origin, and quality. Empty source or target rows are skipped and counted as diagnostics. When --out is omitted, rows are merged into the configured user global knowledge root under memory/adapt/<INPUT_FILE_NAME>.jsonl so repeated imports of the same source stay isolated and idempotent. Use --out to write a specific JSONL file instead."#;

pub(crate) const ADAPT_IMPORT_AFTER_LONG_HELP: &str = r#"Example:
  stringer adapt import --format xt-sst --input ./old.sst --source-locale en --target-locale zh-Hans --game SkyrimSe

Common next step:
  stringer knowledge index rebuild --project-root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;

pub(crate) const KNOWLEDGE_LONG_ABOUT: &str = r#"Knowledge commands provide translation packages and single-text queries with context.

Knowledge sources include terminology TOML, translation-memory JSONL, replacement-rule TOML, and a rebuildable .stringer/indexes/knowledge.sqlite cache. Layers load as built-in, user global, library, then project. annotate, validate, and lookup prefer a fresh index; if the index is missing or stale, they fall back to file-backed knowledge and report knowledge.index_stale."#;

pub(crate) const ANNOTATE_LONG_ABOUT: &str = r#"Write hints into a translation package and optionally auto-fill translations from high-confidence translation memory.

annotate reads the translation package, loads knowledge, removes stale hints written by Stringer's built-in processors, then writes current terminology hints and memory candidates. It preserves existing diagnostics. By default it fills empty translations from high-confidence memory; --skip-fill-memory disables that fill step."#;

pub(crate) const ANNOTATE_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge annotate --project-root ./MyMod --workspace ./translations
  stringer knowledge annotate --workspace ./translations --skip-fill-memory

Agent editing guidance:
  When reading entries/**/*.jsonl, inspect source, context, hints, and diagnostics first.
  When writing translations, change only the translation field; preserve id and source."#;

pub(crate) const VALIDATE_LONG_ABOUT: &str = r#"Recompute translation package diagnostics before finalizing a workspace.

validate does not trust old diagnostics already present in the package. It recomputes diagnostics from the current knowledge files, writes them back, and reports risks without blocking a later workspace finalize."#;

pub(crate) const VALIDATE_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge validate --project-root ./MyMod --workspace ./translations

Common diagnostics:
  term.preferred_missing  preferred terminology was not used
  term.forbidden_used     forbidden translation was used
  placeholder.mismatch    placeholders do not match
  scaleform.newline       Scaleform newline risk
  translation.empty       translation is empty
  memory.conflict         translation conflicts with memory"#;

pub(crate) const LOOKUP_LONG_ABOUT: &str = r#"Search terminology and translation memory for agent-readable translation evidence.

lookup searches loaded knowledge tables by source and target text. By default it performs case-insensitive contains matching across both terminology and memory, ranks normalized exact source matches first, and emits compact results for agent lookup. Use --regex for regex matching and --json for machine-readable results."#;

pub(crate) const LOOKUP_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge lookup --project-root ./MyMod --text "Altmer" --kind plugin --record-type NPC_ --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans --json
  stringer knowledge lookup --text "^(Alt|Bos)mer$" --regex --source memory --field source --json

Available kind values:
  plugin
  strings
  scaleform
  pex

Hints:
  Exact source matches rank before prefix and contains matches.
  Search defaults to --source all --field both --limit 20.
  Plugin entries can use record-type and subrecord to boost context-relevant results."#;

pub(crate) const INDEX_LONG_ABOUT: &str = r#"Knowledge index maintenance commands.

The index is a derived cache, not the source of truth. It can be deleted and rebuilt at any time."#;

pub(crate) const INDEX_REBUILD_LONG_ABOUT: &str = r#"Rebuild <PROJECT_ROOT>/.stringer/indexes/knowledge.sqlite.

rebuild reads the current knowledge-layer files and writes a derived index for terms, memory, rules, and diagnostics. Later annotate, validate, and lookup commands prefer the index when it is fresh."#;

pub(crate) const INDEX_REBUILD_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge index rebuild --project-root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

If knowledge files change often, agents can rebuild before bulk annotate or lookup operations."#;
