# Translation Batches

## Claim

Claim a bounded batch with `workspace_batch_claim` after the workspace has been opened, annotated, and terminology has been organized. Claiming only reserves ownership; it does not return translation entries:

```json
{
  "workspace": "<WORKSPACE>",
  "limit": 50
}
```

The result contains `batch_id`, `claimed_entries`, and `scope`.

## Read

Read claimed entries with `workspace_inspect_batch`:

```json
{
  "workspace": "<WORKSPACE>",
  "batch_id": "<BATCH_ID>",
  "limit": 10,
  "offset": 0
}
```

The result contains `total` and entries with `id`, `source`, optional `translation`, `context`, `hints`, and `diagnostics`. If you apply a partial page, read the batch again from `offset: 0` because applied entries are removed from the remaining claim.

Skip entries already translated by `agent` or `manual` origin unless the user explicitly asks for revision. Memory-prefilled entries can be claimed and improved.

## Translate

For each entry:

- Preserve placeholders, variables, menu tokens, newlines, and punctuation that carry UI meaning.
- Use `hints` first for preferred terms and memory candidates.
- Treat suspected terminology as lookup-required. Before choosing a translation for an uncertain name, proper noun, repeated phrase, or domain term, run `knowledge_lookup` and use the returned evidence with the entry context.
- Do not write a canonical term into the workspace unless it has been verified by lookup evidence and context. Memory hits and prior knowledge are evidence to inspect, not permission to upsert terms by intuition.
- Keep names consistent across entries in the same asset and record type.
- Leave `translation` as `null` or omit it only when no safe translation can be produced.
- Work from entries returned by `workspace_inspect_batch`. Do not open raw `entries/**/*.jsonl` files to translate.

## Apply

Submit one patch for the claimed batch through `workspace_batch_apply`:

```json
{
  "workspace": "<WORKSPACE>",
  "batch_id": "<BATCH_ID>",
  "entries": [
    {
      "id": "<ENTRY_ID>",
      "translation": "<TRANSLATION>"
    }
  ]
}
```

Never apply ids from a different batch. If stopping early, release the batch so remaining entries can be claimed again.
