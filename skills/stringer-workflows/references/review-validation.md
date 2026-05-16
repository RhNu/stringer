# Review and Validation

## Before Review

Run `knowledge_validate` before review.

Validation rewrites diagnostics. Do not edit diagnostics by hand.

## Review Diagnostics

Common diagnostics:

- `term.preferred_missing`: preferred terminology may be absent.
- `term.forbidden_used`: forbidden wording appears.
- `placeholder.mismatch`: placeholders or variables differ.
- `scaleform.newline`: Scaleform line break risk.
- `translation.empty`: translation is missing.
- `memory.conflict`: translation conflicts with memory evidence.

Review entries with `workspace_inspect_diagnostics`, `workspace_inspect_entry`, or `workspace_inspect_batch`. Use `knowledge_lookup` for terminology or memory evidence before changing a translation. Some diagnostics can be acceptable if the context justifies the wording; note the reason in the final report.

## Finalize

Finalize only after validation and review with `workspace_finalize`.

Use a fresh output directory outside the source root. When the MCP host is already operating from the workspace directory, the optional `workspace` argument can be omitted; otherwise pass it explicitly.
