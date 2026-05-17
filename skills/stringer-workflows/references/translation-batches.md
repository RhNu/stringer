# Translation Batches

## Claim

Claim a bounded batch with `workspace_batch_claim` after the workspace has been opened, annotated, and terminology has been organized. Claiming only reserves ownership; it does not return translation entries:

```json
{
  "workspace": "<WORKSPACE>",
  "limit": 50
}
```

The result contains `batch_id`, `revision`, `claimed_entries`, `remaining_claimable`, and `scope`.

## Read

Read claimed entries with `workspace_batch_read`:

```json
{
  "workspace": "<WORKSPACE>",
  "batch_id": "<BATCH_ID>",
  "limit": 10,
  "offset": 0
}
```

The result contains compact rows with `key`, `source`, optional `current_translation`, optional `origin`, `context_label`, hint and diagnostic counts, and diagnostic codes. It intentionally omits full `id`, `context`, `hints`, and `diagnostics` to keep tool output short.

`workspace_batch_detail` returns full rows for found keys and reports unknown requested keys in `missing_keys`. Treat missing keys as a request or stale-batch mistake and re-read the batch before submitting.

Fetch full detail only for keys that need it:

```json
{
  "workspace": "<WORKSPACE>",
  "batch_id": "<BATCH_ID>",
  "keys": ["e001"]
}
```

Skip entries already translated by `agent` or `manual` origin unless the user explicitly asks for revision. Memory-prefilled entries can be claimed and improved.

## Translate

For each entry:

- Preserve placeholders, variables, menu tokens, newlines, and punctuation that carry UI meaning.
- Use compact diagnostic codes to decide which rows need `workspace_batch_detail`.
- Use `hints` from detail first for preferred terms and memory candidates.
- Treat suspected terminology as lookup-required. Before choosing a translation for an uncertain name, proper noun, repeated phrase, or domain term, run `knowledge_lookup` and use the returned evidence with the entry context.
- Do not write a canonical term into the workspace unless it has been verified by lookup evidence and context. Memory hits and prior knowledge are evidence to inspect, not permission to upsert terms by intuition.
- Keep names consistent across entries in the same asset and record type.
- Use `action: "skip"` when an entry does not need translation. A skip must include one `skip_reason`: `not_translatable`, `source_is_target`, `identifier_or_token`, `duplicate_or_obsolete`, or `needs_manual_review`. Do not repeat `source` as `translation` just to complete the batch.
- Use `action: "pending"` when no safe translation or skip decision can be made yet.
- Work from entries returned by batch tools. Do not open raw `entries/**/*.jsonl` files to translate.

## Submit

Submit one request for the claimed batch through `workspace_batch_submit`:

```json
{
  "workspace": "<WORKSPACE>",
  "batch_id": "<BATCH_ID>",
  "revision": 1,
  "entries": [
    {
      "key": "e001",
      "action": "translate",
      "translation": "<TRANSLATION>"
    },
    {
      "key": "e002",
      "action": "skip",
      "skip_reason": "not_translatable"
    },
    {
      "key": "e003",
      "action": "pending"
    }
  ]
}
```

The submit result reports `applied`, `ignored`, or `rejected` per key. If the batch revision is stale, re-read the batch and resubmit against the current revision. If stopping early, release the batch so remaining undecided entries can be claimed again.

For long work or tool-output limits, use `workspace_batch_export` to create an editable JSON submission file under `batch-work/<batch_id>/patch.json`, then submit that file through the CLI. The `patch.json` filename is retained for compatibility with existing exported batch files; `workspace_batch_submit` is the only supported mutation API.
