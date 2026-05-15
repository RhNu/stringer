# Translation Batches

## Claim

Claim a bounded batch:

```powershell
stringer workspace batch claim --workspace <WORKSPACE> --limit 50
```

The result contains `batch_id` and entries with `id`, `source`, optional `translation`, `context`, `hints`, and `diagnostics`.

Skip entries already translated by `agent` or `manual` origin unless the user explicitly asks for revision. Memory-prefilled entries can be claimed and improved.

## Translate

For each entry:

- Preserve placeholders, variables, menu tokens, newlines, and punctuation that carry UI meaning.
- Use `hints` first for preferred terms and memory candidates.
- Use `knowledge lookup` when a source term is ambiguous or repeated.
- Keep names consistent across entries in the same asset and record type.
- Leave `translation` as `null` or omit it only when no safe translation can be produced.

## Apply

Submit one patch for the claimed batch:

```json
{"batch_id":"<BATCH_ID>","entries":[{"id":"<ENTRY_ID>","translation":"<TRANSLATION>"}]}
```

Apply it:

```powershell
stringer workspace batch apply --workspace <WORKSPACE> --input <PATCH_JSON>
```

Never apply ids from a different batch. If stopping early, release the batch so remaining entries can be claimed again.
