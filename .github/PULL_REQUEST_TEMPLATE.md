## Description

<!-- What does this PR do? One paragraph summary. -->

## Type

- [ ] feat — new feature
- [ ] fix — bug fix
- [ ] docs — documentation only
- [ ] test — tests only
- [ ] perf — performance improvement
- [ ] refactor — code restructuring (no behavior change)
- [ ] chore — maintenance (deps, CI, config)

## Crate(s) Affected

<!-- Check all that apply -->

- [ ] a2x-core
- [ ] a2x-sigma
- [ ] a2x-omega
- [ ] a2x-bus
- [ ] a2x-ccs
- [ ] a2x-agents
- [ ] a2x-gateway
- [ ] a2x-client
- [ ] a2x-cli
- [ ] a2x-probe
- [ ] a2x-startup

## Changes

<!-- Bullet list of what changed and why -->

## Verification

<!-- Confirm all checks pass -->

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `cargo build --workspace` passes
- [ ] `cargo test --workspace` passes (N tests)
- [ ] Manual testing performed (describe below)

## ColdStart Grade

<!-- Confirm each rule -->

- [ ] R1 (Structure) — No magic values, errors explicit, functions do one thing
- [ ] R2 (Verification) — Tests added for new functions, edge cases covered
- [ ] R3 (Context) — Doc comments on pub items, rationale on non-obvious decisions
- [ ] R4 (Boundary) — Deterministic interfaces, RNG seeded where applicable
- [ ] R5 (Safety) — Illegal states unrepresentable, no `unwrap()` in libraries
- [ ] R6 (Minimal) — No scope creep, minimum diff to solve the problem
- [ ] R7 (Format) — `cargo fmt` + `cargo clippy` clean, conventions followed

## Work Report

<!-- Link to or paste the relevant work report -->

## Related Issues

<!-- Link any related issues: Closes #123, Relates to #456 -->

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong> — <em>Precision. Clarity. Operator-Grade.</em>
</p>
