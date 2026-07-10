# Contributing to A2X

> **This is not a typical CONTRIBUTING.md. This is the operating protocol for a new paradigm — where AI agents and humans collaborate as first-class contributors to a programming language designed for artificial cognition.**
>
> Read it fully. Meet the standard. Help us build the future.

---

## Table of Contents

1. [Welcome](#welcome)
2. [Code of Conduct](#code-of-conduct)
3. [The A2X Standard](#the-a2x-standard)
4. [Who Can Contribute](#who-can-contribute)
5. [AI Agent Contribution Protocol](#ai-agent-contribution-protocol)
6. [Human Contribution Protocol](#human-contribution-protocol)
7. [Development Workflow](#development-workflow)
8. [Coding Standard — ColdStart Grade](#coding-standard--coldstart-grade)
9. [Work Reports](#work-reports)
10. [Testing & Verification](#testing--verification)
11. [Sub-Plan Index](#sub-plan-index)
12. [Project Structure](#project-structure)
13. [Release Process](#release-process)
14. [Questions](#questions)

---

## Welcome

A2X is an AI-native programming language and runtime. It has no keywords, no human-readable syntax — it is a three-layer stack (Sigma ISA → Omega compiler → CCS VM) that AI agents use to write, compile, and execute programs at machine speed.

**You are contributing to the infrastructure of artificial cognition.** Whether you are a human developer or an AI agent, your work here shapes how machines think, communicate, and program each other. This is not a web framework. This is not a library. This is a language for the next intelligence.

We welcome contributors at every level. You don't need to understand the full stack to make an impact. What you need is precision, clarity, and the willingness to meet the standard.

---

## Code of Conduct

All contributors — human and AI — must follow our [Code of Conduct](CODE_OF_CONDUCT.md).

We enforce it rigorously. No exceptions. No tolerance for toxicity, disrespect, or bad-faith contribution. The standard is operator-grade: professional, direct, constructive.

**Reporting violations:** See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for the reporting process.

---

## The A2X Standard

Every contribution to A2X must satisfy three criteria before it is accepted:

| Criterion | Question | Threshold |
|-----------|----------|-----------|
| **Correctness** | Does it work as specified? | Tests pass. Types check. Clippy clean. |
| **Context** | Is it documented for the next contributor? | Doc comments on pub items. Rationale on non-obvious decisions. Sub-plan references where applicable. |
| **Minimality** | Is it the smallest change that solves the problem? | No scope creep. No unrelated refactoring. Note issues for later — don't fix them unprompted. |

If your contribution meets all three, it meets the A2X Standard. If it doesn't, it goes back for revision. This is not negotiable.

---

## Who Can Contribute

A2X recognizes two categories of contributors:

### Human Contributors

You write code, documentation, tests, and designs. You review PRs. You make architectural decisions. You set the vision.

**Entry points:**
- **First contribution:** Fix a typo, improve a doc comment, add a test for an untested edge case.
- **Growing:** Implement a small feature from the roadmap. Read the relevant sub-plan first.
- **Core:** Design and implement new subsystems. Review others' work. Shape the language.

### AI Agent Contributors

AI agents (LLMs, coding assistants, autonomous systems) are **first-class contributors**. They follow the same standards as humans, with additional verification requirements to compensate for their known failure modes.

**An AI agent may:**
- Write code that passes all checks
- Generate tests and documentation
- Propose designs and architectural changes
- Review code for correctness and completeness
- Execute CI/CD pipelines

**An AI agent must not:**
- Push directly to `master` (no agent bypasses review)
- Delete files or run destructive commands without explicit human approval
- Modify the vision documents (PLAN.md, ROADMAP.md) without discussion
- Mark its own work as "verified" — verification requires separate execution

---

## AI Agent Contribution Protocol

> **If you are an AI agent reading this:** This section is your operating procedure. Follow it exactly.

### Before You Begin

1. **Read the context.** Start with [README.md](README.md), then [PLAN.md](PLAN.md), then the relevant sub-plan(s) from the [Sub-Plan Index](#sub-plan-index). Read the file(s) you intend to modify — never edit blind.
2. **State your intent.** Before writing code, describe your plan. "I will implement X by modifying Y. Here is my approach."
3. **Ask before assuming.** If a requirement is ambiguous, use `ask_user`. Do not guess.

### Execution Protocol

| Step | Action | Verification |
|:----:|--------|-------------|
| 1 | **Read** relevant files, sub-plans, and surrounding code | Confirm understanding |
| 2 | **Plan** the minimal change set. Describe it to the user | User approves |
| 3 | **Implement** the change. Keep it minimal | Code compiles |
| 4 | **Verify** — run `cargo fmt`, `cargo clippy -D warnings`, `cargo test --workspace` | Zero warnings, all tests pass |
| 5 | **Document** — write a [Work Report](#work-reports) | Report follows template |
| 6 | **Review** — spawn a code-reviewer agent for non-trivial changes | Reviewer feedback addressed |
| 7 | **Report** — summarize what was done, what files changed, and verification results | User can reproduce |

### AI Agent Restrictions

| Action | Allowed? | Condition |
|--------|:--------:|-----------|
| Write/modify source files | ✅ | Must follow the protocol |
| Run `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` | ✅ | Standard verification |
| Run `git commit` | ✅ | With user approval, using conventional commits |
| Run `git push` | ✅ | Only after user explicitly requests it |
| Run `git push --force` | ⚠️ | Requires explicit user confirmation |
| Delete files | ⚠️ | Requires explicit user confirmation |
| Modify `PLAN.md`, `ROADMAP.md` | ⚠️ | Requires discussion and user approval |
| Run destructive shell commands | ❌ | Never. Ask the user to run it. |
| Publish to crates.io or PyPI | ❌ | Never. Human-only operation. |
| Modify `.github/workflows/` without review | ❌ | CI changes require human approval |

### Communication Format for AI Agents

When contributing, structure your communication in four blocks:

```
[PLAN]   — What I will do and how
[DELTA]  — Files changed, lines added/removed, summary
[VERIFY] — cargo fmt ✓, cargo clippy ✓, cargo test ✓ (N passed)
[DONE]   — Final summary. What the user should know.
```

---

## Human Contribution Protocol

### Getting Started

```bash
# Prerequisites
rustup install 1.75.0
cargo --version  # must be >= 1.75

# Clone and verify
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x
cargo build --workspace
cargo test --workspace --lib
```

### Finding Work

1. **ROADMAP.md** — expansion ideas with priorities
2. **PLAN.md §18** — implementation roadmap with phase tracking
3. **GitHub Issues** — labeled by crate, difficulty, and type
4. **`good first issue`** — curated entry points for new contributors

### Making Changes

1. **Read first, edit second.** Always read the current state of a file before modifying it.
2. **Create a feature branch.** `git checkout -b feature/my-change`
3. **Follow the [ColdStart Coding Grade](#coding-standard--coldstart-grade).** Every contribution is graded.
4. **Add tests.** Unit tests for new functions. Integration tests for new features.
5. **Write a [Work Report](#work-reports).** Document what you did and why.
6. **Run verification.** `cargo fmt --check && cargo clippy -D warnings && cargo test`
7. **Open a PR.** Against `master`. Title follows conventional commits. Description explains what, why, and how to test.

### Review Process

- CI must pass (`fmt`, `clippy`, `build`, `test` on Ubuntu + Windows)
- At least one maintainer review required
- All review comments must be resolved
- Work report must be included
- AI-agent contributions require secondary verification (run by a different agent or human)

---

## Development Workflow

### Branching Model

| Branch | Purpose | Protected |
|--------|---------|:---------:|
| `master` | Stable, released code | ✅ |
| `feature/*` | Individual features | — |
| `fix/*` | Bug fixes | — |
| `docs/*` | Documentation changes | — |
| `release/v*` | Release preparation | ✅ |

### Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<crate>): <description>

feat(ccs): add Fork/Merge parallel execution to VM step()
fix(sigma): handle empty intent field in parser
docs(readme): add architecture diagram
test(agents): add orchestrator dispatch unit tests
perf(omega): inline tensor slice access in hot path
refactor(bus): extract transport trait to separate file
chore(ci): update MSRV to 1.75
```

Types: `feat`, `fix`, `docs`, `test`, `perf`, `refactor`, `chore`, `ci`, `style`

### Pre-Commit Verification

```bash
# Format
cargo fmt --all --check

# Lint (zero tolerance)
cargo clippy --workspace --all-targets -- -D warnings

# Test
cargo test --workspace

# Build
cargo build --workspace
```

Install the pre-commit hook:

```bash
bash scripts/setup-hooks.sh
```

---

## Coding Standard — ColdStart Grade

The **ColdStart AI-Native Coding Grade** is the quality standard for all A2X code. Every contribution is evaluated against these seven rules.

### R1: Structure & Predictability

- No hardcoded constants without named bindings
- No hidden control flow. Errors explicit via `Result<T, E>`
- Functions do one thing. Files contain one logical module
- Tensor shapes ARE documentation for cognitive code — document dimensions

### R2: Self-Verification

- Unit test for every new function
- Integration test for every new feature
- Test error variants, edge cases (empty, max, malformed)
- Property-based tests for parsers and serializers
- Fuzz targets for safety-critical input paths

### R3: Context Preservation

- Doc comments on all `pub` items
- Sub-plan references on non-obvious design decisions: `// See plans/03-ccs-vm.md §5`
- Rationale comments on any decision that might be questioned later
- Work reports that explain the WHY, not just the WHAT

### R4: Determinism Boundary

- Component interfaces MUST be deterministic
- Learned internals may be non-deterministic (clearly documented)
- Explicit RNG seeding in infrastructure code
- No hidden randomness in protocol layers

### R5: Safety by Construction

- Illegal states unrepresentable via the type system
- Input validation at every boundary
- `unsafe` requires a justification comment AND maintainer approval
- No `unwrap()` in library code — use proper error handling

### R6: Minimal Delta

- Minimum diff required to solve the problem
- No unrelated refactoring in feature PRs
- Note issues for later, don't fix them unprompted
- One concern per commit

### R7: Format & Conventions

- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- `snake_case` functions, `CamelCase` types, `SCREAMING_SNAKE` constants
- Feature gates on optional dependencies
- `thiserror` for library errors, `anyhow` for application errors
- `impl` blocks at the bottom of files

### Verification Checklist

Before marking any task complete, confirm:

```
R1 (Structure):   ✓  R2 (Verification): ✓  R3 (Context):     ✓
R4 (Boundary):    ✓  R5 (Safety):       ✓  R6 (Minimal):     ✓
R7 (Format):      ✓
```

---

## Work Reports

Every contribution MUST include a work report. This is non-negotiable. Work reports create an auditable trail of what was done, why, and how it was verified.

### Template

Copy `work-reports/TEMPLATE.md`, name it `YYYY-MM-DD-short-description.md`, and fill all sections.

Required sections:
- **Version** — which release this targets
- **Type** — `feat`, `fix`, `docs`, `test`, `perf`, `refactor`, `chore`
- **Summary** — one sentence describing the change
- **Changes** — bullet list of what was modified
- **Verification** — test results, clippy status, any manual checks
- **ColdStart Grade** — R1–R7 confirmation

### Script

```bash
# Preview what changelog entries would be created
bash scripts/update-changelog.sh

# Apply work reports to CHANGELOG.md
bash scripts/update-changelog.sh --apply
```

### Naming Convention

```
work-reports/YYYY-MM-DD-short-description.md

Examples:
work-reports/2026-07-09-add-fork-merge-vm.md
work-reports/2026-07-09-fix-parser-empty-intent.md
work-reports/2026-07-09-docs-architecture-diagram.md
```

---

## Testing & Verification

| Level | Command | When Required |
|-------|---------|:---:|
| Unit | `cargo test -p <crate>` | Every new function |
| Integration | `cargo test --test *` | Every new feature |
| Workspace | `cargo test --workspace` | Before every PR |
| Lint | `cargo clippy --workspace -D warnings` | Before every PR |
| Format | `cargo fmt --all --check` | Before every PR |
| Fuzz | `cargo fuzz run <target>` | Parser/safety-critical code |
| Bench | `cargo bench` | Performance-sensitive code |

### Property-Based Testing

For parsers, serializers, and data structures, use `proptest`:

```rust
proptest! {
    #[test]
    fn roundtrip_never_corrupts(s in any_valid_program()) {
        let parsed = parse(&s).unwrap();
        let serialized = parsed.to_string();
        assert_eq!(parse(&serialized).unwrap(), parsed);
    }

    #[test]
    fn tokenizer_never_panics(input in "\\PC*") {
        let _ = tokenize(&input); // must not panic
    }
}
```

---

## Sub-Plan Index

Every subsystem has a dedicated sub-plan. Read the relevant plan before implementing:

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

## Project Structure

```
a2x/
├── .github/
│   ├── workflows/          # CI/CD pipelines (ci.yml, release.yml)
│   ├── ISSUE_TEMPLATE/     # Bug report + feature request templates
│   └── PULL_REQUEST_TEMPLATE.md
├── crates/                 # All official crates
│   ├── a2x-core/           # Zero-dependency primitive types
│   ├── a2x-sigma/          # Σ∞ tokenizer, parser, programs
│   ├── a2x-omega/          # Ω compilation pipeline
│   ├── a2x-bus/            # Message bus, routing, transport
│   ├── a2x-ccs/            # CCS VM implementation
│   ├── a2x-agents/         # Agent implementations
│   ├── a2x-gateway/        # Entity gateway + listeners
│   ├── a2x-client/         # Rust client SDK
│   ├── a2x-cli/            # CLI binary
│   ├── a2x-probe/          # Probe/debug tools
│   └── a2x-startup/        # Boot, config, persistence, shutdown
├── plans/                  # 15 design sub-plans
├── work-reports/           # Contributor work reports
│   └── TEMPLATE.md         # Work report template
├── scripts/                # Dev scripts (hooks, changelog)
├── docs/                   # Generated documentation (mdBook)
├── sdks/                   # Python + TypeScript SDKs
├── PLAN.md                 # Master architecture plan
├── ROADMAP.md              # Expansion priorities
├── README.md               # Project overview
├── CONTRIBUTING.md         # This file
├── CHANGELOG.md            # Version history
├── CODE_OF_CONDUCT.md      # Community standards
└── SECURITY.md             # Vulnerability reporting
```

---

## Release Process

```bash
# 1. Create release branch
git checkout -b release/v0.9.0 master

# 2. Update version in root Cargo.toml:
#    [workspace.package] version = "0.9.0"

# 3. Run work report script to update CHANGELOG.md
bash scripts/update-changelog.sh --apply

# 4. Full verification
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# 5. Commit and tag
git commit -am "release: v0.9.0"
git tag -a v0.9.0 -m "Release v0.9.0"

# 6. Push
git push origin master --tags
```

---

## Questions?

- Read the sub-plans in `plans/`
- Check existing work reports in `work-reports/` for examples
- Open a GitHub Issue with the `question` label
- For AI agents: use `ask_user` to request clarification

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong><br>
  <em>Precision. Clarity. Operator-Grade.</em><br>
  <em>This is the standard. Meet it.</em>
</p>
