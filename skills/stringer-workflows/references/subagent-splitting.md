# Subagent Splitting

Use this guidance only when the host supports independent agents or workers.

## Split Units

Prefer splitting by claimed batch. A worker owns one `batch_id` and submits only that batch.

If batches are not available, split by one entry file reported by `workspace_inspect_files`. Avoid overlapping files.

## Worker Contract

Tell each worker:

- Work only on the assigned batch or entry file.
- Read assigned batch entries with `workspace_batch_read`; claim output is only an ownership summary.
- Use `workspace_batch_detail` for keys that need full `id`, `context`, `hints`, or `diagnostics`.
- Use `knowledge_lookup` before translating suspected or uncertain terms; do not upsert terminology from memory or intuition alone.
- Use inspect and batch tools instead of reading raw workspace files.
- Return the submit summary or the submission JSON if not submitting directly.
- Do not finalize the workspace.

## Coordinator Duties

The coordinator opens and annotates the workspace, organizes terminology before formal translation, creates claims, assigns work, validates after workers finish, and handles finalization.

Run `workspace_batch_count` between rounds to see remaining empty, claimed, translated, and diagnostic counts. Release abandoned claims before finalization; non-forced finalize rejects active claims.
