# Knowledge Lookup

## Rebuild Index

Rebuild after changing project knowledge or importing memory:

```powershell
stringer knowledge index rebuild --project-root <PROJECT_ROOT> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
```

`annotate`, `validate`, and `lookup` prefer a fresh index and fall back to files when needed.

## Edit Project Terms

Prefer term edit commands/tools over direct TOML edits. They create `knowledge/terms/project.toml` by default and restrict custom files to `.toml` paths under `<PROJECT_ROOT>/knowledge/terms/`.

Upsert a term:

```powershell
stringer knowledge term upsert --project-root <PROJECT_ROOT> --id "term:iron_sword" --source "Iron Sword" --target "熟铁剑" --status preferred --alias "Iron Blade" --scope-json '{"game":["SkyrimSe"],"kind":["plugin"],"record_type":["WEAP"]}' --tag weapon --note "Project wording" --json
```

Delete a term:

```powershell
stringer knowledge term delete --project-root <PROJECT_ROOT> --id "term:iron_sword" --json
```

Use `--file knowledge/terms/<NAME>.toml` to target a specific project term file. Use `--rebuild-index` after an edit when subsequent lookup or annotation must use the SQLite index immediately; provide full settings or rely on project settings.

Supported status values are `preferred`, `allowed`, and `forbidden`. Supported scope keys are `game`, `source_locale`, `target_locale`, `kind`, `record_type`, and `asset_path`; CLI `--scope-json` values must be string arrays.

MCP equivalents:

- `knowledge_term_upsert` with `{ "project_root": "...", "term": { "id": "...", "source": "...", "target": "...", "status": "preferred", "scope": { "game": ["SkyrimSe"] } }, "rebuild_index": false }`
- `knowledge_term_delete` with `{ "project_root": "...", "id": "...", "rebuild_index": false }`

## Lookup

Use JSON output for agent evidence:

```powershell
stringer knowledge lookup --project-root <PROJECT_ROOT> --text "Altmer" --kind plugin --record-type NPC_ --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans --json
```

Useful filters:

- `--source memory` or `--source terms`
- `--field source` or `--field target`
- `--regex` for patterns
- `--limit <N>` for compact results

Exact source matches rank ahead of prefix and contains matches. Prefer project and context-relevant evidence over generic matches.

## Adapt Old Resources

Import xTranslator or ESP-ESM Translator resources as memory before bulk annotation:

```powershell
stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --source-locale en --target-locale zh-Hans --game SkyrimSe
```

When `--out` is omitted, the command writes under the standard user knowledge directory.
