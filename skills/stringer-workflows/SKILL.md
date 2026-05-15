---
name: stringer-workflows
description: Use when agent needs to operate Stringer CLI or MCP server for Bethesda mod localization, including opening translation workspaces, translating claimed batches, splitting work across agents, using terminology or translation memory, editing project terminology, reviewing diagnostics, validating workspaces, finalizing overrides, or mapping Stringer CLI commands to MCP tools.
---

# Stringer Workflows

## Core Rules

- Prefer Stringer inspect, batch, and MCP tools. Use inspect tools for read-only review, batch tools for edits, and direct JSONL editing only as a fallback.
- Preserve `id`, `source`, `context`, `hints`, and `diagnostics`. Write translations through `translation` only.
- Use `knowledge lookup` for uncertain names, terminology, repeated phrases, or diagnostic review.
- Use `knowledge term upsert/delete` or MCP `knowledge_term_upsert/delete` for project terminology edits; do not hand-edit term TOML unless the command/tool is unavailable.
- Run `knowledge validate` before `workspace finalize`.
- Treat diagnostics as review inputs. Resolve real risks; do not delete diagnostics manually.

## Default Workflow

1. Open or receive a workspace.
2. Annotate it with project knowledge.
3. Inspect files, remaining work, and diagnostics without reading raw JSONL.
4. Claim a batch, translate with evidence from hints and lookup, then apply the batch.
5. Validate the workspace and review diagnostics.
6. Finalize to an override directory only after validation.

Use explicit game, asset language, and locale settings whenever a command accepts them:

```powershell
--game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
```

## Reference Loading

- For command order and MCP parity, read `references/workflow-overview.md`.
- For translating claimed work, read `references/translation-batches.md`.
- For dividing work across multiple agents, read `references/subagent-splitting.md`.
- For review, validation, and finalize decisions, read `references/review-validation.md`.
- For terminology, memory, and index usage, read `references/knowledge-lookup.md`.

Load only the reference needed for the current task.
