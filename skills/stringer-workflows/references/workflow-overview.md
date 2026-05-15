# Workflow Overview

## CLI Flow

Run commands with full settings when possible:

```powershell
stringer workspace open --root <MOD_ROOT> --workspace <WORKSPACE> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
stringer knowledge annotate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
stringer workspace batch count --workspace <WORKSPACE> --json
stringer workspace batch claim --workspace <WORKSPACE> --limit 50
stringer workspace batch apply --workspace <WORKSPACE> --input <PATCH_JSON>
stringer knowledge validate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
stringer workspace finalize --root <MOD_ROOT> --workspace <WORKSPACE> --override-root <OVERRIDE_ROOT>
```

## MCP Tool Map

Use MCP tools when the host exposes the Stringer server:

| CLI command               | MCP tool                  |
| ------------------------- | ------------------------- |
| `workspace open`          | `workspace_open`          |
| `workspace finalize`      | `workspace_finalize`      |
| `workspace batch count`   | `workspace_batch_count`   |
| `workspace batch claim`   | `workspace_batch_claim`   |
| `workspace batch apply`   | `workspace_batch_apply`   |
| `workspace batch release` | `workspace_batch_release` |
| `adapt import`            | `adapt_import`            |
| `knowledge annotate`      | `knowledge_annotate`      |
| `knowledge validate`      | `knowledge_validate`      |
| `knowledge lookup`        | `knowledge_lookup`        |
| `knowledge index rebuild` | `knowledge_index_rebuild` |
| `knowledge term upsert`   | `knowledge_term_upsert`   |
| `knowledge term delete`   | `knowledge_term_delete`   |

MCP results are structured JSON. Prefer them over parsing CLI text when both are available.

## Workspace Files

`workspace open` writes `workspace.json`, `batches/`, and `entries/**/*.jsonl`.

Agents should normally use batch tools. If direct JSONL inspection is needed, read `source`, `context`, `hints`, and `diagnostics`; write only `translation`.
