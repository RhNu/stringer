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

Review entries with `workspace_inspect_diagnostics`, `workspace_inspect_entry`, `workspace_batch_read`, or `workspace_batch_detail`. Use `knowledge_lookup` for terminology or memory evidence before changing a translation. Some diagnostics can be acceptable if the context justifies the wording; note the reason in the final report.

## Finalize

Finalize only after validation and review with `workspace_finalize`. Non-forced finalize fails if the workspace still has claimable rows, active batch claims, or diagnostics.

Use a fresh output directory outside the source root. Use `force: true` only when the user explicitly accepts finalizing with unfinished rows, active claims, or remaining diagnostics. When the MCP host is already operating from the workspace directory, the optional `workspace` argument can be omitted; otherwise pass it explicitly.
