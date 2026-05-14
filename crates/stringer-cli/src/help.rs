pub(crate) const ROOT_LONG_ABOUT: &str = r#"Stringer is a Bethesda mod localization command-line tool.

It exports translatable mod assets into a JSONL translation package, lets a human or agent edit translations, then writes changed assets into an override directory. Knowledge commands add terminology, translation-memory hints, diagnostics, and single-text lookup support.

Recommended agent workflow:
  1. Run `stringer --help` to understand the whole flow.
  2. Run a subcommand help page, for example `stringer export --help`.
  3. Prefer explicit game, language, and locale arguments so the command does not depend on local machine defaults."#;

pub(crate) const ROOT_AFTER_LONG_HELP: &str = r#"Typical workflow:
  stringer export --root <MOD_ROOT> --out <TRANSLATIONS> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
  stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --out <MOD_ROOT>/knowledge/memory/imported.jsonl --source-locale en --target-locale zh-Hans --game SkyrimSe
  stringer knowledge annotate --root <MOD_ROOT> --translations <TRANSLATIONS>
  # Edit the translation fields in <TRANSLATIONS>/entries/**/*.jsonl.
  stringer knowledge validate --root <MOD_ROOT> --translations <TRANSLATIONS>
  stringer import --root <MOD_ROOT> --translations <TRANSLATIONS> --override-root <OVERRIDE_ROOT>

Default knowledge locations:
  <MOD_ROOT>/knowledge/terms/*.toml
  <MOD_ROOT>/knowledge/memory/*.jsonl
  <MOD_ROOT>/knowledge/rules/*.toml

See README.md for project documentation."#;

pub(crate) const SETTINGS_LONG_HELP: &str = r#"These settings decide how Stringer interprets Bethesda localized assets and translation package locales.

When omitted, commands try to read the default configuration file. For reproducible agent runs, pass these explicitly to export, lookup, and index rebuild:
  --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;

pub(crate) const KNOWLEDGE_ROOTS_LONG_HELP: &str = r#"Knowledge layers are loaded in this order: built-in < global < library < project < override.

The project layer is always <MOD_ROOT>/knowledge. The global layer comes from the default config, project stringer.toml, or --global-knowledge-root. The library layer is global/libraries/<GameRelease>/<target_locale>. The override layer is only used when --override-knowledge-root is passed and is intended for temporary highest-priority terminology or memory overrides."#;

pub(crate) const EXPORT_LONG_ABOUT: &str = r#"Scan a mod root and export an agent-editable translation package.

The output is a directory containing manifest.json and entries/**/*.jsonl. Each JSONL row usually contains id, source, translation, context, hints, and diagnostics. Fresh exports usually leave translation empty for a human or agent to fill.

export currently reads the default config and command-line overrides; it does not read <MOD_ROOT>/stringer.toml."#;

pub(crate) const EXPORT_AFTER_LONG_HELP: &str = r#"Example:
  stringer export --root ./MyMod --out ./translations --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

Output layout:
  <TRANSLATIONS>/manifest.json
  <TRANSLATIONS>/entries/plugin/<asset>/<record_type>.jsonl
  <TRANSLATIONS>/entries/pex/<asset>.jsonl
  <TRANSLATIONS>/entries/scaleform/<asset>.jsonl

Common next step:
  stringer knowledge annotate --root ./MyMod --translations ./translations"#;

pub(crate) const IMPORT_LONG_ABOUT: &str = r#"Read id and translation fields from a translation package, apply them to source mod assets, and write changed files into an override directory.

import ignores hints, diagnostics, and other extension fields. Rows without translation are skipped. The override directory must not be inside the source mod root, which prevents accidental input overwrites."#;

pub(crate) const IMPORT_AFTER_LONG_HELP: &str = r#"Example:
  stringer import --root ./MyMod --translations ./translations --override-root ./StringerOverride

Recommended:
  Run `stringer knowledge validate` before import.
  Point override-root at a fresh directory, then load that directory with a mod manager."#;

pub(crate) const ADAPT_LONG_ABOUT: &str = r#"Adapt external translation resources into Stringer translation memory.

adapt commands do not edit mod assets or translation packages. They read external translator files, normalize usable source/target pairs, and write Stringer memory JSONL that can be placed under <MOD_ROOT>/knowledge/memory."#;

pub(crate) const ADAPT_IMPORT_LONG_ABOUT: &str = r#"Import an external translation resource as Stringer translation memory JSONL.

Supported formats:
  eet       ESP-ESM Translator / EET binary table
  eet-xml   EET XML export
  eet-json  EET JSON or DDS-style export
  xt-sst    xTranslator SST file

The output rows contain id, source, target, source_locale, target_locale, context, origin, and quality. Empty source or target rows are skipped and counted as diagnostics. Use the output as a knowledge/memory/*.jsonl file, then run knowledge annotate or lookup."#;

pub(crate) const ADAPT_IMPORT_AFTER_LONG_HELP: &str = r#"Example:
  stringer adapt import --format xt-sst --input ./old.sst --out ./MyMod/knowledge/memory/old.sst.jsonl --source-locale en --target-locale zh-Hans --game SkyrimSe

Common next step:
  stringer knowledge index rebuild --root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;

pub(crate) const KNOWLEDGE_LONG_ABOUT: &str = r#"Knowledge commands provide translation packages and single-text queries with context.

Knowledge sources include terminology TOML, translation-memory JSONL, replacement-rule TOML, and a rebuildable .stringer/indexes/knowledge.sqlite cache. annotate, validate, and lookup prefer a fresh index; if the index is missing or stale, they fall back to file-backed knowledge and report knowledge.index_stale."#;

pub(crate) const ANNOTATE_LONG_ABOUT: &str = r#"Write hints into a translation package and optionally auto-fill translations from high-confidence translation memory.

annotate reads the translation package, loads knowledge, removes stale hints and diagnostics written by Stringer's built-in processors, then writes current terminology hints, memory candidates, and related diagnostics. It does not overwrite agent-written translations by default; --auto-fill-memory only permits high-confidence memory to fill empty translations."#;

pub(crate) const ANNOTATE_AFTER_LONG_HELP: &str = r#"Examples:
  stringer knowledge annotate --root ./MyMod --translations ./translations
  stringer knowledge annotate --root ./MyMod --translations ./translations --auto-fill-memory

Agent editing guidance:
  When reading entries/**/*.jsonl, inspect source, context, hints, and diagnostics first.
  When writing translations, change only the translation field; preserve id and source."#;

pub(crate) const VALIDATE_LONG_ABOUT: &str = r#"Recompute translation package diagnostics before import.

validate does not trust old diagnostics already present in the package. It recomputes diagnostics from the current knowledge files, writes them back, and reports risks without blocking a later import."#;

pub(crate) const VALIDATE_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge validate --root ./MyMod --translations ./translations

Common diagnostics:
  term.preferred_missing  preferred terminology was not used
  term.forbidden_used     forbidden translation was used
  placeholder.mismatch    placeholders do not match
  scaleform.newline       Scaleform newline risk
  translation.empty       translation is empty
  memory.conflict         translation conflicts with memory"#;

pub(crate) const LOOKUP_LONG_ABOUT: &str = r#"Look up knowledge hints for a single source text, intended for agents translating one string at a time.

lookup builds a temporary entry from text, kind, record-type, subrecord, game, language, and locale settings, then matches terminology and memory. Use --json for machine-readable hints and diagnostics."#;

pub(crate) const LOOKUP_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge lookup --root ./MyMod --text "Iron Sword" --kind plugin --record-type WEAP --subrecord FULL --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans --json

Available kind values:
  plugin
  strings
  scaleform
  pex

Hints:
  Plugin entries usually benefit from record-type and subrecord.
  Scaleform entries usually only need kind=scaleform and text.
  PEX is not part of the first knowledge-enrichment path, but lookup accepts kind=pex to keep the interface uniform."#;

pub(crate) const INDEX_LONG_ABOUT: &str = r#"Knowledge index maintenance commands.

The index is a derived cache, not the source of truth. It can be deleted and rebuilt at any time."#;

pub(crate) const INDEX_REBUILD_LONG_ABOUT: &str = r#"Rebuild <MOD_ROOT>/.stringer/indexes/knowledge.sqlite.

rebuild reads the current knowledge-layer files and writes a derived index for terms, memory, rules, and diagnostics. Later annotate, validate, and lookup commands prefer the index when it is fresh."#;

pub(crate) const INDEX_REBUILD_AFTER_LONG_HELP: &str = r#"Example:
  stringer knowledge index rebuild --root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

If knowledge files change often, agents can rebuild before bulk annotate or lookup operations."#;
