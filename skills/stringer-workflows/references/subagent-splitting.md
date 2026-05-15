# Subagent Splitting

Use this guidance only when the host supports independent agents or workers.

## Split Units

Prefer splitting by claimed batch. A worker owns one `batch_id` and applies only that batch.

If batches are not available, split by one entry file from `workspace.json`. Avoid overlapping files.

## Worker Contract

Tell each worker:

- Work only on the assigned batch or entry file.
- Preserve `id`, `source`, `context`, `hints`, and `diagnostics`.
- Use knowledge lookup for uncertain terms.
- Return the applied summary or the patch JSON if not applying directly.
- Do not finalize the workspace.

## Coordinator Duties

The coordinator opens and annotates the workspace, creates claims, assigns work, validates after workers finish, and handles finalization.

Run `workspace batch count --json` between rounds to see remaining empty, claimed, translated, and diagnostic counts.
