# AGENTS.md

# Workspace Layout

Update this section when folder layout changes.

- `/docs`: documentaion root.
- `/crates`: Rust workspace crates.
- `/xtask`: workspace automation and maintenance scripts.
- `/temp`: temp folder containing temporary test fixture, reference codes; outside the folder should never reference or mention the specific sub path inside.

# Code Rules

## Rust

- The 4 things before submitting work: `fmt`, `clippy`, `test`, `xtask line-budget`
- Name tests after the behavior.
- Keep Rust source and test files at or below 850 lines. Check this mechanically with `cargo xtask line-budget`; split by responsibility before a file grows past that budget.
- Keep repeated test helpers (mock, tempfile, etc.) in shared module or crate.
- Maintain minimalist `lib.rs` and `main.rs`. Decouple core logic into dedicated modules; the entry points should only act as a routing or dispatching entry point.
