# Work Report

> **Version:** <version>  
> **Type:** <feat | fix | docs | test | perf | refactor | chore>  
> **Date:** YYYY-MM-DD  
> **Author:** <name | agent-id>
>
> *Work reports are **required for AI-agent contributions** and **encouraged for larger human contributions** (new features, new crates, subsystem changes). They create an auditable trail of what was done, why, and how it was verified — and become the entries in `CHANGELOG.md` at release time.*
>
> *For small human-authored fixes (typos, one-liners, docs), a good PR description is enough — skip this template.*

---

## Summary

*One sentence. What changed? Why does it matter?*

---

## Changes

*Bullet list. What files were modified? What did each change accomplish?*

- `crates/<crate>/src/<file>.rs` — <description>
- `crates/<crate>/tests/<test>.rs` — <description>

---

## Verification

*How was this change verified? Fill all applicable.*

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✓ / ✗ |
| `cargo clippy -D warnings` | ✓ / ✗ |
| `cargo build --workspace` | ✓ / ✗ |
| `cargo test --workspace` | N passed, 0 failed |
| Manual testing | <describe any manual verification> |

---

## ColdStart Grade

*Confirm each rule. If a rule does not apply, mark it N/A with a reason.*

| Rule | Status | Notes |
|:----:|:------:|-------|
| R1 (Structure) | ✓ | |
| R2 (Verification) | ✓ | |
| R3 (Context) | ✓ | |
| R4 (Boundary) | ✓ | |
| R5 (Safety) | ✓ | |
| R6 (Minimal) | ✓ | |
| R7 (Format) | ✓ | |

---

## Sub-Plan References

*List any sub-plans or design documents that informed this change.*

- `plans/XX-name.md` §<section> — <what was referenced>

---

## AI Agent Declaration

*For AI agent contributions only. Humans delete this section.*

| Declaration | Confirmation |
|-------------|:-----------:|
| All files were read before editing | ✓ |
| Changes are minimal — no scope creep | ✓ |
| Verification was executed (not assumed) | ✓ |
| User approved the plan before implementation | ✓ |
| This report accurately reflects the work done | ✓ |

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong><br>
  <em>Precision. Clarity. Operator-Grade.</em>
</p>
