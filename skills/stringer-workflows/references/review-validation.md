# Review and Validation

## Before Review

Run validation:

```powershell
stringer knowledge validate --project-root <PROJECT_ROOT> --workspace <WORKSPACE>
```

Validation rewrites diagnostics. Do not edit diagnostics by hand.

## Review Diagnostics

Common diagnostics:

- `term.preferred_missing`: preferred terminology may be absent.
- `term.forbidden_used`: forbidden wording appears.
- `placeholder.mismatch`: placeholders or variables differ.
- `scaleform.newline`: Scaleform line break risk.
- `translation.empty`: translation is missing.
- `memory.conflict`: translation conflicts with memory evidence.

Review the entry, its context, hints, and lookup evidence before changing a translation. Some diagnostics can be acceptable if the context justifies the wording; note the reason in the final report.

## Finalize

Finalize only after validation and review:

```powershell
stringer workspace finalize --root <MOD_ROOT> --workspace <WORKSPACE> --override-root <OVERRIDE_ROOT>
```

Use a fresh override directory outside the source mod root.
