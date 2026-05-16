# Workflow Overview

## Tool Flow

Use Stringer MCP tools directly. They return structured JSON, so consume their fields instead of reading raw workspace files or unstructured text output.

1. `workspace_open`: create or refresh the workspace from a read-only source root. Pass explicit `settings` when known.
2. `knowledge_annotate`: write terminology, memory, and diagnostic hints into workspace rows.
3. `workspace_inspect_files`, `workspace_batch_count`, and `workspace_inspect_diagnostics`: understand scope, remaining work, and risks without raw-file reads.
4. Terminology pass before formal translation: use `knowledge_lookup` for suspected, recurring, or ambiguous source terms, then `knowledge_term_upsert` or `knowledge_term_delete` only for terms verified by lookup evidence and entry context.
5. `knowledge_annotate`: run again after terminology changes so later claims include updated hints.
6. `workspace_batch_claim`: claim a bounded set of entries for translation ownership.
7. `workspace_inspect_batch`: read claimed entries in pages.
8. `workspace_batch_apply`: apply translations for that exact claimed batch.
9. `knowledge_validate`: recompute diagnostics.
10. `workspace_inspect_diagnostics`: review any remaining warnings or errors with entry context.
11. `workspace_finalize`: write translated assets to a fresh output directory only after validation.

When the MCP host is already operating from the workspace directory, the optional `workspace` argument can be omitted. Otherwise pass it explicitly.

## Workspace Files

`workspace_open` writes `workspace.json`, `batches/`, and `entries/**/*.jsonl`. Treat those as tool-managed storage. Agents should use inspect and batch tools instead of direct file reads or edits.

Workspace knowledge lives under `knowledge/`, global user knowledge lives beside the user config, and each layer has its own derived `index.sqlite`. Workspace knowledge ids override global ids. Lookup, annotate, and validate refresh missing, stale, or corrupt indexes automatically; use `knowledge_index_rebuild` when you want to refresh both layers explicitly.

Legacy `manifest.json` workspaces are not read by the current workspace format. Recreate legacy workspaces with `workspace_open`.
