# Knowledge Lookup

## Rebuild Index

Rebuild after changing workspace knowledge or importing memory:

```powershell
stringer knowledge index rebuild --workspace <WORKSPACE>
```

Knowledge is layered as global first, then workspace. Workspace term, rule, and memory ids override matching global ids. `annotate`, `validate`, and `lookup` automatically refresh missing or stale layer indexes before using them.

## Edit Workspace Terms

Prefer term edit commands/tools over direct TOML edits. They create `knowledge/terms/workspace.toml` by default and restrict custom files to `.toml` paths under `<WORKSPACE>/knowledge/terms/`.

Upsert a term:

```powershell
stringer knowledge term upsert --workspace <WORKSPACE> --id "term:iron_sword" --source "Iron Sword" --target "熟铁剑" --status preferred --alias "Iron Blade" --scope-json '{"game":["SkyrimSe"],"kind":["plugin"],"record_type":["WEAP"]}' --tag weapon --note "Workspace wording" --json
```

Delete a term:

```powershell
stringer knowledge term delete --workspace <WORKSPACE> --id "term:iron_sword" --json
```

Use `--file knowledge/terms/<NAME>.toml` to target a specific workspace term file. Use `--rebuild-index` after an edit when you want the workspace layer index rebuilt immediately instead of on the next lookup, annotate, or validate run.

Supported status values are `preferred`, `allowed`, and `forbidden`. Supported scope keys are `game`, `source_locale`, `target_locale`, `kind`, `record_type`, and `asset_path`; CLI `--scope-json` values must be string arrays.

MCP equivalents:

- `knowledge_term_upsert` with `{ "workspace": "...", "terms": [{ "id": "...", "source": "...", "target": "...", "status": "preferred", "scope": { "game": ["SkyrimSe"] } }], "rebuild_index": false }`
- `knowledge_term_delete` with `{ "workspace": "...", "id": "...", "rebuild_index": false }`

## Lookup

Use JSON output for agent evidence:

```powershell
stringer knowledge lookup --workspace <WORKSPACE> --text "Altmer" --kind plugin --record-type NPC_ --json
```

Useful filters:

- `--source memory` or `--source terms`
- `--field source` or `--field target`
- `--regex` for patterns
- `--limit <N>` for compact results

Exact source matches rank ahead of prefix and contains matches. Prefer workspace and context-relevant evidence over generic matches.

## Adapt Old Resources

Import xTranslator or ESP-ESM Translator resources as memory before bulk annotation:

```powershell
stringer adapt import --format xt-sst --input <OLD_TRANSLATION.sst> --source-locale en --target-locale zh-Hans --game SkyrimSe
```

When `--out` is omitted, the command writes under the standard user knowledge directory.
