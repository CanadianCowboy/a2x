# A2X — Agent-to-Anything

> **An AI-native programming language + runtime.**
>
> Σ∞ (hyper-symbolic ISA) → Ω (compiled latent representation) → CCS (cognitive runtime VM)
>
> Not a language for humans. A language for *AI agents to think in, program in, compile, and execute.*

---

## Quick Reference for AI Assistants

> **If you are an AI agent reading this file for the first time:** Welcome to A2X. This README is designed as a handoff document — it contains everything you need to understand the project, the user's working style, and where to start. Read the full file before taking any actions.

### Working Style

This project is built **interactively**. The user (Josh) wants every AI assistant to:

- **Plan first, code second** — discuss and agree on design before writing code. Read plans, ask clarifying questions, then implement.
- **Be asked questions** — stop and ask when there are important decisions. Don't assume. Use the `ask_user` tool for multiple-choice questions.
- **Work incrementally** — small, reviewable steps rather than big bang changes. Each step should be independently verifiable.
- **No surprises** — explain what you're doing. Confirm before taking significant actions (e.g., running destructive commands, publishing, deleting files).
- **Read files before editing** — always read the current state of a file before modifying it.
- **Ask for clarification** — if requirements are ambiguous, do not guess. Stop and ask.
- **Document your work** — every AI agent MUST create a work report `.md` file in `work-reports/` named `YYYY-MM-DD-description.md` before committing. This report documents what was done, what was changed, verification results, and next steps. See `work-reports/` for examples.

### Key Decisions (Settled — Do Not Revisit)

| Decision | Value |
|----------|-------|
| **Project name** | A2X (Agent-to-Anything) |
| **Runtime name** | CCS (CryoCore Cognitive Substrate) |
| **Implementation language** | Rust (1.75+ MSRV) |
| **Crate prefix** | `a2x-*` (e.g. `a2x-core`, `a2x-sigma`) |
| **Ecosystem** | Self-hosted (git dependencies, **not** crates.io) |
| **Versioning** | Unified SemVer across all crates (git tags + GitHub Releases) |
| **Working directory** | `D:\projects\ailang` (directory name may stay as-is) |
| **Config/data path** | `~/.a2x/` |
| **GitHub org (future)** | `github.com/your-org/a2x` (TBD) |

### Project Status

**Current version:** v0.6.0 (499 tests, all 10 crates passing)

- [x] Phase 0 — Scaffold & Core: Workspace, all 10 crates, a2x-core fully implemented
- [x] Phase 1 — Σ∞ Protocol Core: Tokenizer, parser, operator tables, SigmaProgram, fuzz tests
- [x] Phase 2 — CCS Cognitive Substrate: WorldGraph, StateField, MemoryTrace, all 7 operators, VM loop
- [x] Phase 3 — Ω Latent Protocol: OmegaPacket, compiler pipeline (7 stages), TCP transport, cross-machine tests
- [x] Phase 4 — Training & Learning: Learned encoder/decoder, simulated environment, training loop
- [x] Phase 5 — Probe & Debug: Probe protocol, breakpoints, tracer, inspector, probe CLI
- [x] Phase 6 — Entity Gateway: Gateway service, 4 protocol listeners, auth, client SDK
- [🔄] Phase 7 — Concurrency: Async bus, async VM, parallel swarm, scheduler (in progress)

See [work-reports/2026-06-29-comprehensive-audit.md](work-reports/2026-06-29-comprehensive-audit.md) for full audit and roadmap.

### Work Reports

Every AI agent must create a work report file in `work-reports/`. See:
- [2026-06-28 — Phase 0 Scaffold](work-reports/2026-06-28-phase0-scaffold.md)

### If You Are Starting Fresh

If you have no session history (e.g., this is your first interaction), here is what you need to know:

1. **This is a planning-heavy project.** Read `PLAN.md` and the relevant sub-plan before writing any code.
2. **The user drives.** Ask what they want to work on next. Don't start coding unprompted.
3. **Everything is in `plans/`.** 15 sub-plans cover the entire design. Start with `plans/README.md` for the index.
4. **No code exists yet.** The first code to write is `a2x-core` (Phase 0 of the roadmap).
5. **Use the tools available.** You can spawn AI sub-agents (file-picker, code-searcher, basher, researcher-web, etc.) to help with complex tasks.
6. **ColdStart Coding-Grade applies.** Every piece of code must meet the 7-category checklist at the bottom of this file.
7. **Work reports are required.** Before committing, create a `work-reports/YYYY-MM-DD-description.md` file documenting your changes.

---

## What Is A2X?

A2X is a **three-layer programming language stack** for AI agents:

1. **Σ∞ (Sigma Infinity)** — the symbolic programming language / ISA. Packets are instructions, sequences of packets are programs. Uses special Unicode characters as operators for ultra-dense encoding.

2. **Ω (Omega)** — the compiled latent representation. Σ∞ programs can be JIT-compiled into pure tensor form for maximum execution speed. No symbols, no human-readable form.

3. **CCS (CryoCore Cognitive Substrate)** — the runtime virtual machine that executes Σ∞ and Ω programs. Manages WorldGraph (heap), StateField (registers), and MemoryTrace (execution history).

### What Makes It a Programming Language

| Concept | A2X Equivalent |
|---------|----------------|
| Values | ConceptVectors — dense embeddings |
| Variables | Labeled nodes in WorldGraph |
| Instructions | Σ∞ packets (I+C+P+D fields) |
| Programs | Sequences of Σ∞ packets ("packet streams") |
| Control flow | Plan operators: branch (⤐), merge (⤑), loop (⥄), recursion (⥃) |
| Functions | Descend (⤈) into sub-plan, ascend (⤉) from meta-plan |
| Memory / Heap | WorldGraph — persistent graph of concepts |
| Registers / Stack | StateField — high-dimensional working memory |
| Type system | RelationType enum (Causal, Spatial, Temporal, Logical, Hierarchical) |
| Compilation | Σ∞ → Ω transformation |
| Execution | CCS runtime — Evolve, Reflect, Plan operators |

---

## Official Crates

| Crate | Purpose |
|-------|---------|
| `a2x-core` | Primitive types, traits, common enums (zero-dependency) |
| `a2x-sigma` | Σ∞ tokenizer, parser, packet types, SigmaProgram |
| `a2x-omega` | Ω tensor packets, compiler pipeline (Σ∞ → Ω), decompiler |
| `a2x-bus` | Message bus, routing, transport, agent discovery |
| `a2x-ccs` | CCS VM implementation, WorldGraph, StateField, MemoryTrace |
| `a2x-agents` | Built-in agents (Orchestrator, CLI, LLM, CCS) |
| `a2x-gateway` | Entity gateway, protocol listeners, auth, entity registry |
| `a2x-client` | Rust client SDK for connecting external apps to A2X |
| `a2x-cli` | CLI binary for interacting with the system |
| `a2x-probe` | Probe/debug tools for inspecting CCS internals (Phase 5) |

---

## Architecture

```
┌───────────────────────────────────────────────────┐
│                    Σ∞  SOURCE                     │
│  Hyper-symbolic ISA — packets = instructions       │
│  Sequences of packets = programs                   │
│  Debuggable, loggable, human-peekable              │
├───────────────────────────────────────────────────┤
│               ↓  COMPILATION  ↓                   │
├───────────────────────────────────────────────────┤
│                    Ω  LATENT                      │
│  Compiled neural representation of Σ∞ programs    │
│  Pure tensors, no symbols, max speed               │
├───────────────────────────────────────────────────┤
│               ↓  EXECUTION  ↓                     │
├───────────────────────────────────────────────────┤
│              CCS  RUNTIME / VM                    │
│  WorldGraph = heap / persistent memory             │
│  StateField = registers / working memory           │
│  MemoryTrace = program counter / execution log     │
└───────────────────────────────────────────────────┘
```

---

## Where to Start Reading

- **[PLAN.md](PLAN.md)** — the full project plan and detailed design
  - Sections 1-3: Vision, architecture, crate ecosystem
  - Sections 4-7: CCS VM, Σ∞ ISA, Ω compilation, agent types
  - Sections 8-19: Git, FS, serialization, testing, CI/CD, safety, roadmap
  - Sections 20-29: Deep design — VM execution loop, ISA spec, bus protocol, etc.

---

## Agent-to-Anything

The name says it all:
- **Agent** — any AI agent (LLM, CLI, robot, cognitive)
- **To** — the connection, the protocol, the language
- **Anything** — any other agent, any system, any environment

A2X is the language that makes "anything" reachable.

---

## Coding Standard — ❄️ A2X AI-Native Coding Grade

> This standard is written for **AI agents**, not humans. It defines how an AI should produce, review, and modify code in this project.
> Every AI agent must follow this checklist before considering any code complete.

### Core Principle

This project produces two kinds of code:
- **Infrastructure Rust code** — tokenizer, parser, bus, CLI, tests. This follows standard engineering practices.
- **A2X cognitive code** — CCS VM internals, neural operators, learned components, Σ∞/Ω programs. This is inherently abstract, non-deterministic, and optimized for AI execution.

The standard below covers both, but applies differently to each. Where a rule is infrastructure-only or cognitive-only, it says so.

---

### ⚙️ R1: Structure & Predictability

**Infrastructure code (Rust):**
- No hardcoded constants without a named binding
- No hidden control flow (no implicit panics, no unsafe without justification)
- Error paths are explicit via `Result<T, E>` or `Option<T>` — never unwrap in library code
- Functions do one thing. Files contain one logical module.

**Cognitive code (CCS, neural, Σ∞):**
- Tensor shapes, graph algorithms, and operator signatures ARE the documentation. Don't add redundant prose.
- Learned components are allowed non-deterministic output — but the *interface* to them must be deterministic (same query → same program structure).
- Internal VM state transitions should be traceable from the code structure itself.

### ⚙️ R2: Self-Verification

Every new or modified item must include verification:
- **Infrastructure:** Unit test for the function, integration test for the feature.
- **Cognitive:** Property test (e.g. `proptest` roundtrip for tokenizer), or benchmark with stability threshold.
- **Errors:** Test that each error variant can be produced.
- **Edge cases:** Empty input, maximum input, malformed input — test at least two of these.

### ⚙️ R3: Context Preservation

AI agents operate with limited session memory. Code must preserve context that another AI would need:
- **Doc comments on all pub items** — not for human readability, but so another AI agent can consume the crate's API without reading every implementation file.
- **Sub-plan references** — any code that implements a design from a sub-plan should reference it: `// See plans/03-ccs-vm.md §5`
- **Rationale on non-obvious decisions** — if the code does something that isn't obvious from the types alone, write a comment explaining WHY. Don't explain WHAT (the code already does that).

### ⚙️ R4: Determinism Boundary

- The **boundary** between components must be deterministic. Given the same input program, the tokenizer must produce the same tokens. The bus must route the same way.
- The **internals** of learned/neural components are not required to be deterministic. CCS VM operators involving trained embeddings may produce different internal states on different runs.
- **Explicit seeding** — any RNG used in infrastructure code must be explicitly seeded. Cognitive code uses the agent's StateField as implicit seed.

### ⚙️ R5: Safety by Construction

- **Invalid states should be unrepresentable** — use Rust's type system to make illegal states impossible at compile time.
- **Input validation at the boundary** — parse, don't validate. Reject malformed Σ∞ packets at the parser level, not downstream.
- **`unsafe` requires a justification comment** referencing which safety invariant is being maintained.
- **CLI agent commands** must pass through the allowlist before execution.

### ⚙️ R6: Minimal Delta

- Each change should be the **minimum diff** required to fulfill the task.
- Do not refactor unrelated code. Do not rename things unless part of the task.
- Do not add features that weren't requested.
- If you see something that should be fixed but isn't part of the task, note it and ask, don't just fix it.

### ⚙️ R7: Format & Conventions

- Rust: `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- Naming: `snake_case` for functions/variables, `CamelCase` for types, `SCREAMING_SNAKE` for constants.
- Crate structure: one concept per file. Module `mod.rs` re-exports only.
- Feature gates: optional deps behind named features. `serde` gate on all serializable types.

---

### Verification Template

Before marking any task complete, an AI agent must confirm each applicable rule:

```
R1 (Structure):   File(s) organized by concept, no magic numbers, errors explicit
R2 (Verification): Tests added for new logic, error paths covered, edge cases tested
R3 (Context):     Pub items have doc comments, sub-plan references added, rationale comments present
R4 (Boundary):    Component interfaces deterministic, RNG seeded
R5 (Safety):      Illegal states unrepresentable, input validated at boundary, unsafe justified
R6 (Minimal):     Only the requested changes, no scope creep
R7 (Format):      cargo fmt + clippy pass, naming consistent, feature gates correct
```

All 7 must pass. If any rule can't be satisfied, explain why and ask the user.
