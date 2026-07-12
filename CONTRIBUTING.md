# Contributing to A2X

Thanks for wanting to help. A2X is an AI-native programming language and runtime — humans and AI agents are both welcome. This doc keeps the human path short.

> **AI contributors** (Claude, GPT, Copilot, Cursor, Aider, Codex, etc.) — read [AGENTS.md](AGENTS.md) instead. You have a stricter protocol.

---

## Quick start

```bash
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x
cargo build --workspace
cargo test --workspace
```

Rust 1.75+ required (`rustup install 1.75.0`).

---

## Find something to work on

- **[ROADMAP.md](ROADMAP.md)** — expansion ideas with priorities
- **[GitHub Issues](https://github.com/CanadianCowboy/a2x/issues)** — bugs and features
- **[good first issue](https://github.com/CanadianCowboy/a2x/labels/good%20first%20issue)** — curated newcomer picks
- **[PLAN.md](PLAN.md) §18** — implementation roadmap, if you want the big picture

If a direction in [ROADMAP.md](ROADMAP.md) interests you, just start. No permission required.

---

## Make your change

```bash
git checkout -b feature/short-description   # or fix/... , docs/...
# edit files
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git commit -m "feat(crate): what changed"
git push -u origin feature/short-description
```

Then open a PR against `master`.

### Commit style

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(ccs): add Fork/Merge parallel execution to VM step()
fix(sigma): handle empty intent field in parser
docs(readme): fix broken link
```

Types: `feat`, `fix`, `docs`, `test`, `perf`, `refactor`, `chore`, `ci`.

---

## Sign the CLA (one-time)

On your first PR, our CLA bot will comment with a link to [CLA.md](CLA.md) and instructions. You reply with one line:

> I have read the CLA Document and I hereby sign the CLA

That's it. One-time per contributor, tracked by GitHub username. See [CLA.md](CLA.md) for what you're agreeing to (short version: you grant the project a broad license to your contribution, and you confirm the work is yours to give).

---

## What we check on PRs

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, and `cargo test` on Ubuntu + Windows. If any fail, the PR won't merge until they pass. Everything else is human review.

A maintainer will review within a few days. For larger changes, expect a conversation about approach — that's normal, not rejection.

---

## Work reports (optional for humans, required for AI)

For any change larger than a small fix, a work report in `work-reports/YYYY-MM-DD-short-description.md` is really appreciated — it becomes the changelog entry and helps future contributors understand your reasoning. Not required for humans, though. Template: [work-reports/TEMPLATE.md](work-reports/TEMPLATE.md).

AI-agent contributions **do** require a work report — see [AGENTS.md](AGENTS.md).

---

## Coding style

- Follow the surrounding code. If it's `snake_case`, use `snake_case`.
- Doc comments (`///`) on public items when reasonable
- `unwrap()` is OK in tests and CLI code, not in library code — prefer `?` or a real error
- One concern per commit; unrelated refactors go in their own PR
- New public API? Add a test.

The full "AI-Native Coding Grade" (R1–R7) applies to AI-authored code and lives in [AGENTS.md](AGENTS.md). Humans can read it if curious but aren't graded on it.

---

## Where things live

```
a2x/
├── crates/               # 11 workspace crates (a2x-core, a2x-sigma, ...)
├── plans/                # 15 design sub-plans (read if you're changing that subsystem)
├── work-reports/         # Contribution history
├── scripts/              # Dev scripts (hooks, changelog)
├── docs/                 # Generated docs (mdBook)
├── sdks/                 # Python + TypeScript SDKs
├── PLAN.md               # Architecture plan
├── ROADMAP.md            # Expansion priorities
├── AGENTS.md             # Protocol for AI contributors
└── CLA.md                # Contributor License Agreement
```

### Sub-plan index

Reading the relevant sub-plan before changing a subsystem saves everyone time. Not required for typos or one-line fixes.

| # | Plan | Covers |
|:-:|------|--------|
| 01 | [Sigma Language](plans/01-sigma-language.md) | Σ∞ ISA, tokenizer, parser |
| 02 | [Omega Compiler](plans/02-omega-compiler.md) | Compilation pipeline, optimizer |
| 03 | [CCS VM](plans/03-ccs-vm.md) | VM loop, memory model |
| 04 | [Bus Protocol](plans/04-bus.md) | Message bus, transport |
| 05 | [Agents](plans/05-agents.md) | Agent trait, lifecycle |
| 06 | [Entity Gateway](plans/06-entity-gateway.md) | Gateway, listeners, auth |
| 07 | [Probe & Debug](plans/07-probe.md) | Probe protocol, tracer |
| 08 | [Ecosystem](plans/08-ecosystem.md) | Crate structure, CI/CD |
| 09 | [Core Types](plans/09-core-types.md) | Primitives, traits, enums |
| 10 | [Concurrency](plans/10-concurrency.md) | Async model, scheduling |
| 11 | [Startup & Shutdown](plans/11-startup-shutdown.md) | Boot, config, persistence |
| 12 | [Security](plans/12-security.md) | Auth, encryption, sandboxing |
| 13 | [Documentation](plans/13-documentation.md) | Doc standards, mdbook |
| 14 | [Resilience](plans/14-resilience.md) | Fault tolerance, recovery |
| 15 | [WASM Support](plans/15-wasm.md) | Browser agents, WASM VM |

---

## Reporting bugs and vulnerabilities

- **Regular bugs:** [open an issue](https://github.com/CanadianCowboy/a2x/issues/new/choose)
- **Security vulnerabilities:** please DO NOT open a public issue. See [SECURITY.md](SECURITY.md) for the private-advisory process.

---

## Code of Conduct

By participating, you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). TL;DR: professional, direct, constructive. No harassment.

---

## Questions?

- Open a [Discussion](https://github.com/CanadianCowboy/a2x/discussions) for a general question
- Open an [Issue](https://github.com/CanadianCowboy/a2x/issues/new/choose) for a bug or feature request
- Direct maintainer contact: **mail@josh-lynes.com**

---

Thanks for contributing. If this doc got in your way, open a PR against it — that's the most welcome kind.
