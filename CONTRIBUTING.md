# Contributing to A2X

> **This document is for both human contributors AND AI agents. Every AI assistant should read this file before making any changes to the codebase.**

---

## First: Read the README

Before contributing, read [README.md](README.md) in full — especially the "Quick Reference for AI Assistants" section. It contains the project's working style, key decisions, and the ColdStart Coding-Grade standard that all code must meet.

---

## Code of Conduct

All contributors must follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be respectful, inclusive, and constructive.

---

## How to Pick Up Work

1. **Check the roadmap** — Read `PLAN.md` §18 (Implementation Roadmap) to see what phase we're in.
2. **Read the relevant sub-plan** — Each subsystem has a dedicated sub-plan in `plans/`. Read it before writing code.
3. **Talk to the user** — Use `ask_user` to confirm what to work on next. Don't assume.
4. **Ask clarifying questions** — If the user says "implement X" but X is ambiguous, ask for specifics before coding.

### Decision-Making Guide for AI Agents

| Situation | Action |
|-----------|--------|
| Ambiguous requirement | Ask the user with `ask_user` (multiple choice) |
| Small, obvious fix (< 10 lines) | Just do it, mention it in summary |
| Medium change (new function/module) | Describe your plan, ask for approval |
| Large feature (new crate/agent) | Read the sub-plan(s), propose a step-by-step plan, get approval |
| Destructive operation (delete, force push) | ALWAYS ask first |
| Adding a dependency | Check `Cargo.toml` convention, use `cargo add` |
| Unsure about a design decision | Check `PLAN.md` + sub-plans first, then ask |

---

## Getting Started

### Prerequisites

- Rust 1.75+ (`rustup install 1.75.0`)
- Git
- No external services (self-hosted ecosystem)

### First Time Setup

```bash
# Clone (if not already)
git clone <repo-url>
cd a2x

# Check Rust version
rustc --version  # Must be >= 1.75

# Verify everything works
cargo build --workspace  # (future, once code exists)
cargo test --workspace   # (future, once tests exist)
```

---

## Workflow

### Branching

| Branch | Purpose | Who |
|--------|---------|-----|
| `main` | Stable, released code | Core maintainers only |
| `develop` | Integration branch for next release | All contributors |
| `feature/*` | Individual features | Feature authors |
| `release/v*` | Release preparation (before tagging) | Maintainers |
| `hotfix/*` | Urgent bug fixes off `main` | Anyone |

### Creating a Feature Branch

```bash
git checkout develop
git pull origin develop
git checkout -b feature/my-feature
```

### Making Changes

1. **Read first, edit second** — always read the current state of a file before modifying it.
2. **Follow the crate structure** — code goes in `crates/<crate-name>/src/`.
3. **Follow R1–R7 (AI-Native Coding Grade)** — see the full standard in [README.md](README.md#coding-standard--️-a2x-ai-native-coding-grade). Run the verification template before marking any task complete.
4. **Keep changes minimal** — do exactly what the task requires, no more.
5. **Add tests** — unit tests for new functions, integration tests for new features.
6. **Update the sub-plan** if you change a design decision — keep `plans/*.md` in sync with the code.

### Committing

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<crate>): <description>

feat(core): add ConceptVector::cosine_similarity
fix(sigma): handle empty intent field in parser
docs(plan): update ISA opcode table
test(agents): add orchestrator dispatch unit tests
perf(omega): inline tensor slice access
```

Types: `feat`, `fix`, `docs`, `test`, `perf`, `refactor`, `chore`, `ci`.

### Before Opening a PR

Run these checks:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

### Opening a PR

1. Push your branch: `git push origin feature/my-feature`
2. Open a PR against `develop` (or `main` for hotfixes)
3. Title follows conventional commits
4. Description explains: what changed, why, how to test
5. CI must pass
6. At least one maintainer review required

---

## Coding Standard

All code must meet **ColdStart Coding-Grade** — the full checklist is in [README.md](README.md#-coldstart-coding-grade).

### AI-Native Coding Standard (R1–R7)

The full standard with per-rule details is in [README.md](README.md#coding-standard--️-a2x-ai-native-coding-grade).

| Rule | Focus | Quick Reference |
|:----:|-------|----------------|
| **R1** | Structure & Predictability | No magic values, errors explicit, tensor shapes ARE docs for cognitive code |
| **R2** | Self-Verification | Unit/integration tests for infrastructure, property tests + benchmarks for cognitive |
| **R3** | Context Preservation | Doc comments for AI consumption, sub-plan references, rationale on non-obvious decisions |
| **R4** | Determinism Boundary | Component interfaces must be deterministic. Learned internals need not be. Seed RNG. |
| **R5** | Safety by Construction | Type-safe states, validate at boundary, `unsafe` always justified |
| **R6** | Minimal Delta | Only the requested change, no scope creep, note extra issues don't fix them |
| **R7** | Format & Conventions | `cargo fmt` + `cargo clippy -D warnings`, snake_case/CamelCase, feature gates |

**Verification template** (paste and confirm before marking any task complete):
```
R1 (Structure):   ✓  R2 (Verification): ✓  R3 (Context):     ✓
R4 (Boundary):    ✓  R5 (Safety):       ✓  R6 (Minimal):     ✓
R7 (Format):      ✓
```

### Rust-Specific Conventions

- Use `thiserror` for library error types, `anyhow` for applications
- All public items must have doc comments
- Use `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` for serialization
- Feature gates: `default = ["std"]`, optional deps behind named features
- `impl` blocks at the bottom of files (after struct/enum definitions)
- Module files in `src/` — one concept per file (e.g., `tokenizer.rs`, `parser.rs`)

---

## Sub-Plan Index

The entire architecture is documented in 15 sub-plans. When implementing something, read the relevant sub-plan first:

| # | File | Covers |
|:-:|------|--------|
| 01 | [Sigma Language](plans/01-sigma-language.md) | Σ∞ ISA, operators, tokenizer, parser, SigmaProgram |
| 02 | [Ω Compiler](plans/02-omega-compiler.md) | Ω packet shape, compilation pipeline, optimizer |
| 03 | [CCS VM](plans/03-ccs-vm.md) | CCS runtime, VM loop, memory model, ISA encoding |
| 04 | [Bus Protocol](plans/04-bus.md) | Message bus, transport, discovery, routing |
| 05 | [Agents](plans/05-agents.md) | Agent trait, lifecycle, safety model, error types |
| 06 | [Entity Gateway](plans/06-entity-gateway.md) | Gateway, protocol listeners, auth, client SDKs |
| 07 | [Probe & Debug](plans/07-probe.md) | Probe protocol, breakpoints, channel separation |
| 08 | [Ecosystem](plans/08-ecosystem.md) | Crate structure, CI/CD, versioning, testing |
| 09 | [Core Types](plans/09-core-types.md) | a2x-core: ConceptVector, traits, enums, errors |
| 10 | [Concurrency](plans/10-concurrency.md) | Async model, multi-program scheduling, thread safety |
| 11 | [Startup & Shutdown](plans/11-startup-shutdown.md) | Boot order, config, state persistence |
| 12 | [Security](plans/12-security.md) | Auth, encryption, permissions, sandboxing |
| 13 | [Documentation](plans/13-documentation.md) | Doc tools, mdbook, standards |
| 14 | [Resilience](plans/14-resilience.md) | Graceful degradation, fault tolerance, crash recovery |
| 15 | [WASM Support](plans/15-wasm.md) | Browser-based agents, WASM VM, web dashboards |

---

## Project Structure

```
a2x/
├── .github/workflows/    # CI/CD pipelines
├── crates/               # All official crates
│   ├── a2x-core/         # Zero-dependency primitive types
│   ├── a2x-sigma/        # Σ∞ tokenizer, parser, programs
│   ├── a2x-omega/        # Ω compilation pipeline
│   ├── a2x-bus/          # Message bus, routing
│   ├── a2x-ccs/          # CCS VM implementation
│   ├── a2x-agents/       # Agent implementations
│   ├── a2x-gateway/      # Entity gateway
│   ├── a2x-client/       # Rust client SDK
│   ├── a2x-cli/          # CLI binary
│   └── a2x-probe/        # Probe/debug tools
├── plans/                # 15 design sub-plans
├── examples/             # Example programs
├── scripts/              # Build/dev scripts
├── docs/                 # Generated documentation
├── PLAN.md               # Master plan
├── README.md             # Project overview + AI context
└── CONTRIBUTING.md       # This file
```

---

## Communication for AI Agents

When contributing as an AI agent, follow this communication protocol:

1. **Start by reading** — README.md, PLAN.md, the relevant sub-plan, and the file(s) you need to modify.
2. **State your plan** — "I'm going to do X by Y. Here's my approach."
3. **Use `ask_user` for decisions** — don't guess between A and B. Present options.
4. **Summarize after each step** — "Done. I changed these files: [...]. Next I'll [...]."
5. **Report blockers** — "I can't proceed because [reason]. Can you clarify?"
6. **Use sub-agents** — For complex tasks, spawn code-searcher, file-picker, basher, or researcher-web agents to gather context in parallel.

---

## Testing

| Level | Command | When |
|-------|---------|------|
| Unit | `cargo test -p <crate>` | After adding/modifying a function |
| Integration | `cargo test --test *` | After adding a feature |
| All | `cargo test --workspace` | Before PR |
| Lint | `cargo clippy --workspace -D warnings` | Before PR |
| Format | `cargo fmt --all --check` | Before PR |
| Fuzz | `cargo fuzz run <target>` | For parser/safety-critical code |
| Bench | `cargo bench` | For performance-sensitive code |

---

## Work Reports & Changelog

### How It Works

Every contributor writes a **work report** for their changes. These stay local (gitignored). The changelog is updated from them.

```
You write:  work-reports/2026-07-08-my-feature.md  (local, not tracked)
Then run:   ./scripts/update-changelog.sh --apply
Commits:    CHANGELOG.md is updated and versioned
```

### Writing a Work Report

1. Copy `work-reports/TEMPLATE.md`
2. Name it `YYYY-MM-DD-short-description.md`
3. Fill in version, type, summary, changes, verification
4. Run `./scripts/update-changelog.sh` to preview
5. Run `./scripts/update-changelog.sh --apply` to update CHANGELOG.md
6. Commit CHANGELOG.md along with your code

### Changelog Script

```bash
# Preview what would be added
./scripts/update-changelog.sh

# Actually update CHANGELOG.md
./scripts/update-changelog.sh --apply
```

The script scans `work-reports/` for new reports (tracked by `.last-processed` marker), extracts version/type/title from each, and inserts entries into CHANGELOG.md under `[unreleased]`.

### Manual Changelog Update

If you prefer to update CHANGELOG.md manually, add entries under the appropriate section:

```markdown
## [unreleased]

### Added
- New feature description

### Fixed
- Bug fix description
```

When releasing, replace `[unreleased]` with the version tag.

```bash
# 1. Create release branch
git checkout -b release/v0.1.0 develop

# 2. Update version in root Cargo.toml
#    [workspace.package] version = "0.1.0"

# 3. Update all crate Cargo.toml files to match

# 4. Run full checks
cargo build --workspace && cargo test --workspace

# 5. Merge to main
git checkout main
git merge --no-ff release/v0.1.0

# 6. Tag
git tag -a v0.1.0 -m "Release v0.1.0"

# 7. Push
git push origin main --tags
```

---

## Questions?

If you're unsure about anything:
- Check the sub-plans in `plans/`
- Ask the user using `ask_user`
- Open a GitHub Discussion (future)

---

*This file is part of the A2X project. See [PLAN.md](PLAN.md) for the full architecture.*
