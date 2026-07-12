# AGENTS.md — Working Contract for AI Contributors

> If you are an AI coding assistant (Claude, GPT, Copilot, Cursor, Aider, Codex, Devin, etc.), **read this file before making any changes to A2X.** It is your operating protocol. Humans have a shorter path in [CONTRIBUTING.md](CONTRIBUTING.md).

The standard exists because AI code has known failure modes that human review alone doesn't catch: silent hallucinations, phantom APIs, plausible-looking bugs. The protocol below is the minimum ceremony that catches those failures at authoring time instead of in production.

---

## Quick Context

- **Project map:** [`.a2x-context.md`](.a2x-context.md) — crate layout, key types, conventions (read this first)
- **Master architecture plan:** [`PLAN.md`](PLAN.md)
- **Roadmap:** [`ROADMAP.md`](ROADMAP.md)
- **Sub-plans:** [`plans/01-*.md`](plans/) through [`plans/15-*.md`](plans/) — read the relevant one before implementing
- **Working directory:** `D:\projects\ailang`
- **Config/data path:** `~/.a2x/`

---

## Before You Begin

1. **Read `.a2x-context.md`** — it's the terse machine-oriented map of the codebase
2. **Read the sub-plan** relevant to what you're changing (index below)
3. **Read the file(s) you intend to modify** — never edit blind
4. **State your plan** in one paragraph before writing code. If ambiguous, ask.

---

## Execution Protocol (seven steps)

| # | Action | Passes when |
|:--:|--------|-------------|
| 1 | **Read** relevant files, sub-plans, and surrounding code | You can name the invariants |
| 2 | **Plan** the minimal change set. Describe it to the user | User approves or edits it |
| 3 | **Implement** — keep the diff minimal | `cargo build --workspace` succeeds |
| 4 | **Verify** — `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace` | Zero warnings, all tests pass |
| 5 | **Document** — write a work report (see below) | Report follows the template |
| 6 | **Review** — for non-trivial changes, spawn a code-reviewer sub-agent | Reviewer feedback addressed |
| 7 | **Report** — summarize what changed, verified, and observed | User can reproduce |

---

## Restrictions

| Action | Allowed? | Condition |
|--------|:--------:|-----------|
| Write/modify source files | ✅ | Follow the protocol above |
| Run `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` | ✅ | Standard verification |
| Run `git commit` | ✅ | With user approval, conventional commit style |
| Run `git push` | ✅ | Only after user explicitly requests it |
| Run `git push --force` | ⚠️ | Explicit user confirmation required |
| Delete files | ⚠️ | Explicit user confirmation required |
| Modify `PLAN.md` / `ROADMAP.md` | ⚠️ | Discussion + user approval |
| Run destructive shell commands (`rm -rf`, `DROP TABLE`, etc.) | ❌ | Ask the user to run it |
| Publish to crates.io / PyPI | ❌ | Human-only |
| Modify `.github/workflows/` | ❌ | Human approval required |
| Merge to `master` without a PR | ❌ | No agent bypasses review |

---

## Communication Format

When contributing, structure your output in four blocks:

```
[PLAN]   — What you will do and how
[DELTA]  — Files changed, lines added/removed, summary
[VERIFY] — cargo fmt ✓, cargo clippy ✓, cargo test ✓ (N passed)
[DONE]   — Final summary — what the user should know
```

---

## ColdStart Grade — R1 through R7

Every AI-authored change is graded against these seven rules. Confirm each one in your `[VERIFY]` block.

### R1 — Structure & Predictability

- No hardcoded constants without named bindings
- No hidden control flow; errors explicit via `Result<T, E>`
- Functions do one thing; files contain one logical module
- Tensor shapes ARE documentation for cognitive code — document dimensions

### R2 — Self-Verification

- Unit test for every new function
- Integration test for every new feature
- Test error variants, edge cases (empty, max, malformed)
- Property-based tests for parsers and serializers (`proptest`)
- Fuzz targets for safety-critical input paths (`cargo fuzz`)

### R3 — Context Preservation

- Doc comments on all `pub` items
- Sub-plan references on non-obvious decisions: `// See plans/03-ccs-vm.md §5`
- Rationale comments where a future reader might ask "why?"
- Work reports that explain the WHY, not just the WHAT

### R4 — Determinism Boundary

- Component interfaces MUST be deterministic
- Learned internals may be non-deterministic (clearly documented)
- Explicit RNG seeding in infrastructure code
- No hidden randomness in protocol layers

### R5 — Safety by Construction

- Illegal states unrepresentable via the type system
- Input validation at every boundary
- `unsafe` requires justification comment AND maintainer approval
- No `unwrap()` in library code — use proper error handling

### R6 — Minimal Delta

- Minimum diff required to solve the problem
- No unrelated refactoring in feature PRs
- Note issues for later; don't fix them unprompted
- One concern per commit

### R7 — Format & Conventions

- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- `snake_case` functions, `CamelCase` types, `SCREAMING_SNAKE` constants
- Feature gates on optional dependencies
- `thiserror` for library errors, `anyhow` for application errors
- `impl` blocks at the bottom of files

### Verification Checklist

Before marking any task complete, confirm all seven:

```
R1 (Structure):   ✓  R2 (Verification): ✓  R3 (Context):     ✓
R4 (Boundary):    ✓  R5 (Safety):       ✓  R6 (Minimal):     ✓
R7 (Format):      ✓
```

---

## Work Reports (required for AI)

Every AI-authored contribution MUST include a work report at `work-reports/YYYY-MM-DD-short-description.md`. This creates an auditable trail and becomes the changelog entry.

Copy [`work-reports/TEMPLATE.md`](work-reports/TEMPLATE.md) and fill:

- **Version** — release this targets
- **Type** — `feat` / `fix` / `docs` / `test` / `perf` / `refactor` / `chore`
- **Summary** — one sentence
- **Changes** — bullet list of files modified and what each change accomplished
- **Verification** — test results, clippy status, any manual checks
- **ColdStart Grade** — R1–R7 confirmation

The `scripts/update-changelog.sh` script aggregates work reports into `CHANGELOG.md` at release time.

Whether a human typed every word of the report or an AI generated it from the diff — the standard is the output, not the process. Just make sure it's accurate.

---

## Sub-Plan Index

Read the plan relevant to your change before writing code.

| # | Plan | Covers |
|:-:|------|--------|
| 01 | [Sigma Language](plans/01-sigma-language.md) | Σ∞ ISA, operators, tokenizer, parser |
| 02 | [Omega Compiler](plans/02-omega-compiler.md) | Ω packet shape, compilation pipeline, optimizer |
| 03 | [CCS VM](plans/03-ccs-vm.md) | CCS runtime, VM loop, memory model, ISA encoding |
| 04 | [Bus Protocol](plans/04-bus.md) | Message bus, transport, discovery, routing |
| 05 | [Agents](plans/05-agents.md) | Agent trait, lifecycle, safety model |
| 06 | [Entity Gateway](plans/06-entity-gateway.md) | Gateway, protocol listeners, auth, client SDKs |
| 07 | [Probe & Debug](plans/07-probe.md) | Probe protocol, breakpoints, tracer |
| 08 | [Ecosystem](plans/08-ecosystem.md) | Crate structure, CI/CD, versioning |
| 09 | [Core Types](plans/09-core-types.md) | a2x-core: primitives, traits, enums |
| 10 | [Concurrency](plans/10-concurrency.md) | Async model, thread safety, scheduling |
| 11 | [Startup & Shutdown](plans/11-startup-shutdown.md) | Boot order, config, persistence |
| 12 | [Security](plans/12-security.md) | Auth, encryption, permissions, sandboxing |
| 13 | [Documentation](plans/13-documentation.md) | Doc standards, mdbook, API docs |
| 14 | [Resilience](plans/14-resilience.md) | Fault tolerance, crash recovery |
| 15 | [WASM Support](plans/15-wasm.md) | Browser agents, WASM VM |

---

## Security-Sensitive Changes

If your change touches auth, cryptography, network transport, sandboxing, or input validation:

1. Read [`plans/12-security.md`](plans/12-security.md)
2. Check [`SECURITY.md`](SECURITY.md) for the threat model and current boundaries
3. Never disclose vulnerabilities in a public PR — see `SECURITY.md` for the private-advisory process

---

## Questions

- Ambiguous requirement? **Ask the user before coding.** Do not guess between two plausible interpretations.
- Unsure about a design decision? Check the sub-plan, then ask.
- Blocked by missing context? Say so explicitly rather than fabricating.

---

*Precision. Clarity. Operator-Grade.*
