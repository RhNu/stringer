# AGENTS.md

# Workspace Layout

Update this section when folder layout changes.

- `/docs`: documentaion root.
- `/crates`: Rust workspace crates.
- `/temp`: temp folder containing temporary test fixture, reference codes; outside the folder should never reference or mention the specific sub path inside.

# Code Rules

## Rust

- The 3 things before submitting work: `fmt`, `clippy`, `test`
- Name tests after the behavior.
- Keep Rust source and test files at or below 850 lines. Split by responsibility before a file grows past that budget.
- Keep repeated test helpers (mock, tempfile, etc.) in shared module or crate.
