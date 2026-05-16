# Workflow Overview

## CLI Flow

Run commands with full settings when possible:

```powershell
stringer workspace open --source-root <SOURCE_ROOT> --workspace <WORKSPACE> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
stringer knowledge annotate --workspace <WORKSPACE>
stringer workspace batch count --workspace <WORKSPACE> --json
stringer workspace inspect diagnostics --workspace <WORKSPACE> --severity warning
stringer workspace batch claim --workspace <WORKSPACE> --limit 50
stringer workspace batch apply --workspace <WORKSPACE> --input <PATCH_JSON>
stringer knowledge validate --workspace <WORKSPACE>
stringer workspace finalize --workspace <WORKSPACE> --output <OUTPUT_DIR>
```

When the agent is already running in the workspace directory, omit `--workspace`; it defaults to `.`.

## MCP Tool Map

Use MCP tools when the host exposes the Stringer server:

| CLI command                     | MCP tool                          |
| ------------------------------- | --------------------------------- |
| `workspace open`                | `workspace_open`                  |
| `workspace finalize`            | `workspace_finalize`              |
| `workspace upgrade`             | no MCP tool; CLI placeholder only |
| `workspace batch count`         | `workspace_batch_count`           |
| `workspace batch claim`         | `workspace_batch_claim`           |
| `workspace batch apply`         | `workspace_batch_apply`           |
| `workspace batch release`       | `workspace_batch_release`         |
| `workspace inspect files`       | `workspace_inspect_files`         |
| `workspace inspect entries`     | `workspace_inspect_entries`       |
| `workspace inspect entry`       | `workspace_inspect_entry`         |
| `workspace inspect batch`       | `workspace_inspect_batch`         |
| `workspace inspect diagnostics` | `workspace_inspect_diagnostics`   |
| `adapt import`                  | `adapt_import`                    |
| `knowledge annotate`            | `knowledge_annotate`              |
| `knowledge validate`            | `knowledge_validate`              |
| `knowledge lookup`              | `knowledge_lookup`                |
| `knowledge index rebuild`       | `knowledge_index_rebuild`         |
| `knowledge term upsert`         | `knowledge_term_upsert`           |
| `knowledge term delete`         | `knowledge_term_delete`           |

MCP results are structured JSON. Prefer them over parsing CLI text when both are available.

## Workspace Files

`workspace open` writes `workspace.json`, `batches/`, and `entries/**/*.jsonl`. Legacy `manifest.json` workspaces are not read by v4 commands; `workspace upgrade` currently reports that migration is not implemented, so recreate legacy workspaces with `workspace open`.

Workspace knowledge lives under `knowledge/`, global user knowledge lives beside the user config, and each layer has its own derived `index.sqlite`. Workspace knowledge ids override global ids. Lookup, annotate, and validate refresh missing, stale, or corrupt indexes automatically; use `knowledge index rebuild` when you want to refresh both layers explicitly.

Agents should normally use inspect tools for read-only review and batch tools for edits. If direct JSONL inspection is needed as a fallback, read `source`, `context`, `hints`, and `diagnostics`; write only `translation`.
