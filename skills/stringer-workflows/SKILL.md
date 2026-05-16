---
name: stringer-workflows
description: Use when an agent needs to operate Stringer MCP tools for Bethesda mod localization, including opening translation workspaces, annotating knowledge, preparing terminology, translating claimed batches, splitting work across agents, reviewing diagnostics, validating workspaces, or finalizing outputs.
---

# Stringer Workflows

## Core Rules

- Use Stringer MCP tools for workspace operations. Do not read `workspace.json`, `entries/**/*.jsonl`, `batches/*.json`, or knowledge TOML directly unless the user explicitly asks for raw-file debugging.
- Use inspect tools for read-only review, batch tools for translation edits, and knowledge tools for terminology, memory, annotation, validation, and index work.
- Preserve `id`, `source`, `context`, `hints`, and `diagnostics`. Write translated text through `translation`; use `skip: true` only for entries that do not need translation.
- Use `knowledge_lookup` before choosing translations for suspected terms, uncertain names, repeated phrases, or diagnostic review.
- Use `knowledge_term_upsert` and `knowledge_term_delete` for workspace terminology edits; do not hand-edit term TOML.
- Only write terminology that has been verified through lookup evidence and entry context. Do not create or update terms from knowledge-base intuition or memory alone.
- Run `knowledge_validate` before `workspace_finalize`.
- Treat diagnostics as review inputs. Resolve real risks; do not delete diagnostics manually.

## Default Workflow

1. Open or receive a workspace with `workspace_open` or an existing workspace path.
2. Annotate it with global and workspace knowledge using `knowledge_annotate`.
3. Inspect files, remaining work, and diagnostics with `workspace_inspect_*` and `workspace_batch_count`; do not read raw workspace files.
4. Organize terminology before formal translation: use `hints`, diagnostics, and `knowledge_lookup` to identify repeated or risky terms, then update workspace terminology with `knowledge_term_upsert` or `knowledge_term_delete`.
5. Re-run `knowledge_annotate` after terminology changes so later batches carry the updated hints.
6. Start formal batch translation with `workspace_batch_claim`, read compact rows with `workspace_batch_read`, fetch full rows with `workspace_batch_detail` only when needed, and write through `workspace_batch_submit`.
7. Validate with `knowledge_validate`, review diagnostics with inspect tools, and repeat focused fixes if needed.
8. Finalize with `workspace_finalize` only after validation and diagnostic review.

If `stringer.toml` does not contain explicit settings, use explicit game, asset language, and locale settings in `workspace_open.settings` whenever they are known:

```json
{
  "settings": {
    "game_release": "SkyrimSe",
    "asset_language": "English",
    "source_locale": "en",
    "target_locale": "zh-Hans"
  }
}
```

## Reference Loading

- For MCP tool order and workspace lifecycle, read `references/workflow-overview.md`.
- For translating claimed work, read `references/translation-batches.md`.
- For dividing work across multiple agents, read `references/subagent-splitting.md`.
- For review, validation, and finalize decisions, read `references/review-validation.md`.
- For terminology, memory, and index usage, read `references/knowledge-lookup.md`.

Load only the reference needed for the current task.
