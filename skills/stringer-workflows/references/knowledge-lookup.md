# Knowledge Lookup

## Rebuild Index

Rebuild after changing project knowledge or importing memory:

```powershell
stringer knowledge index rebuild --project-root <PROJECT_ROOT> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
```

`annotate`, `validate`, and `lookup` prefer a fresh index and fall back to files when needed.

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

When `--out` is omitted, the command writes to the configured user global knowledge root.
