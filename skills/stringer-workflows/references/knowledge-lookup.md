# Knowledge Lookup

## Rebuild Index

Use `knowledge_index_rebuild` after changing workspace knowledge or importing memory when you want to refresh both knowledge layers immediately.

Knowledge is layered as global first, then workspace. Workspace term, rule, and memory ids override matching global ids. `annotate`, `validate`, and `lookup` automatically refresh missing, stale, or corrupt layer indexes before using them.

## Edit Workspace Terms

Use term edit tools instead of direct TOML edits. They create `knowledge/terms/workspace.toml` by default and restrict custom files to `.toml` paths under `<WORKSPACE>/knowledge/terms/`.

Before upserting any term, verify it with `knowledge_lookup` and relevant entry context. Do not create or replace workspace terms from memory hits, prior knowledge, or knowledge-base intuition alone.

Upsert terms with `knowledge_term_upsert`:

```json
{
  "workspace": "<WORKSPACE>",
  "terms": [
    {
      "id": "term:iron_sword",
      "source": "Iron Sword",
      "target": "熟铁剑",
      "aliases": ["Iron Blade"],
      "status": "preferred",
      "scope": {
        "game": ["SkyrimSe"],
        "kind": ["plugin"],
        "record_type": ["WEAP"]
      },
      "tags": ["weapon"],
      "note": "Workspace wording"
    }
  ],
  "rebuild_index": false
}
```

Delete a term with `knowledge_term_delete`:

```json
{
  "workspace": "<WORKSPACE>",
  "id": "term:iron_sword",
  "rebuild_index": false
}
```

Use `file: "knowledge/terms/<NAME>.toml"` to target a specific workspace term file. Set `rebuild_index: true` when you want the workspace layer index rebuilt immediately instead of on the next lookup, annotate, or validate run.

Supported status values are `preferred`, `allowed`, and `forbidden`. Supported scope keys are `game`, `source_locale`, `target_locale`, `kind`, `record_type`, and `asset_path`; scope values must be string arrays.

After terminology edits, run `knowledge_annotate` before claiming formal translation batches so updated terms appear in `hints`.

## Lookup

Use `knowledge_lookup` for agent evidence:

```json
{
  "workspace": "<WORKSPACE>",
  "text": "Altmer",
  "kind": "plugin",
  "record_type": "NPC_",
  "limit": 20
}
```

Useful filters:

- `source: "memory"` or `source: "terms"`
- `field: "source"` or `field: "target"`
- `regex: true` for patterns
- `limit` for compact results

Exact source matches rank ahead of prefix and contains matches. Prefer workspace and context-relevant evidence over generic matches.

## Adapt Old Resources

Import xTranslator or ESP-ESM Translator resources as memory before bulk annotation with `adapt_import`:

```json
{
  "format": "xt-sst",
  "input": "<OLD_TRANSLATION.sst>",
  "source_locale": "en",
  "target_locale": "zh-Hans",
  "game": "SkyrimSe"
}
```

When `out` is omitted, the tool writes under the standard user knowledge directory.
