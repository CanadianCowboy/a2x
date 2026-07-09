# A2X — Agent-to-Anything

> **An AI-native programming language + runtime.**
>
> Sigma-infinity (hyper-symbolic ISA) → Omega (compiled latent representation) → CCS (cognitive runtime VM)
>
> Not a language for humans. A language for *AI agents to think in, program in, compile, and execute.*

---

## Quick Start (Alpha)

```bash
# Clone and build
git clone https://github.com/your-org/a2x
cd a2x
cargo build --release

# Launch the dashboard (one command)
cargo run --release -p a2x-cli -- dashboard

# Or with Ollama chat enabled
A2X_CHAT_BACKEND=ollama A2X_CHAT_MODEL=llama3.2 cargo run --release -p a2x-cli -- dashboard

# Try the interactive shell
cargo run --release -p a2x-cli -- shell

# Parse and execute a Sigma program
cargo run --release -p a2x-cli -- run --program "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭"
```

Open `http://127.0.0.1:8778` in your browser to see the live dashboard.

### Available CLI Commands

| Command | Description |
|---------|-------------|
| `a2x dashboard` | Launch the web dashboard (gateway + WebSocket UI) |
| `a2x shell` | Interactive Sigma REPL with colored output |
| `a2x monitor` | Bus traffic viewer — agents, capabilities, dispatch |
| `a2x run -p <expr>` | Parse and execute a Sigma program |
| `a2x parse -p <expr>` | Parse and display a Sigma program |
| `a2x agents` | List registered agents with capabilities |
| `a2x probe <id>` | Inspect an agent's internal state |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `A2X_CHAT_BACKEND` | `none` | `ollama` or `openai` |
| `A2X_CHAT_MODEL` | `llama3.2` | Model name |
| `A2X_CHAT_API_URL` | `http://localhost:11434/v1/chat/completions` | LLM API endpoint |
| `A2X_CHAT_API_KEY` | (empty) | API key for OpenAI |
| `A2X_CHAT_CONTEXT_TOKENS` | `32768` | Context window size |
| `A2X_HTTP_PORT` | `8778` | Dashboard/API port |
| `A2X_API_KEY` | (none) | API key for authenticated access |

### What's in the Dashboard

- **Agents panel** — live agent cards with capabilities
- **WorldGraph** — force-directed graph of concept nodes and relations
- **StateField heatmap** — tensor region visualization
- **VM inspector** — status, region readout, memory trace
- **Chat tab** — streaming chat with Ollama/OpenAI, model switching, conversation persistence
- **Sigma playground** — write and execute Sigma programs inline
- **Bus traffic log** — real-time event stream
- **Context usage bar** — tokens used / remaining

---

## Quick Reference for AI Assistants

> **If you are an AI agent reading this file for the first time:** Welcome to A2X. This README is designed as a handoff document — it contains everything you need to understand the project, the user's working style, and where to start. Read the full file before taking any actions.

### Working Style

This project is built **interactively**. The user (Josh) wants every AI assistant to:

- **Plan first, code second** — discuss and agree on design before writing code. Read plans, ask clarifying questions, then implement.
- **Be asked questions** — stop and ask when there are important decisions. Don't assume.
- **Work incrementally** — small, reviewable steps rather than big bang changes. Each step should be independently verifiable.
- **No surprises** — explain what you're doing. Confirm before taking significant actions (e.g., running destructive commands, publishing, deleting files).
- **Read files before editing** — always read the current state of a file before modifying it.
- **Ask for clarification** — if requirements are ambiguous, do not guess. Stop and ask.
- **Document your work** — every AI agent MUST create a work report `.md` file in `work-reports/` named `YYYY-MM-DD-description.md` before committing.

### Key Decisions (Settled — Do Not Revisit)

| Decision | Value |
|----------|-------|
| **Project name** | A2X (Agent-to-Anything) |
| **Runtime name** | CCS (CryoCore Cognitive Substrate) |
| **Implementation language** | Rust (1.75+ MSRV) |
| **Crate prefix** | `a2x-*` (e.g. `a2x-core`, `a2x-sigma`) |
| **Ecosystem** | Self-hosted (git dependencies, **not** crates.io) |
| **Versioning** | Unified SemVer across all crates |
| **Working directory** | Any directory (clone with `git clone <url> a2x`) |
| **Config/data path** | `~/.a2x/` |

### Project Status

**Current version:** v0.9.0-alpha (70+ tests, 12 crates building)

- [x] Phase 0 — Scaffold & Core: Workspace, all 12 crates
- [x] Phase 1 — Sigma Protocol Core: Tokenizer, parser, operator tables
- [x] Phase 2 — CCS Cognitive Substrate: WorldGraph, StateField, MemoryTrace, 7 operators
- [x] Phase 3 — Omega Latent Protocol: Compiler pipeline, TCP transport
- [x] Phase 4 — Training & Learning: Learned encoder/decoder
- [x] Phase 5 — Probe & Debug: Probe protocol, breakpoints, tracer
- [x] Phase 6 — Entity Gateway: Gateway service, 4 listeners, auth, client SDK
- [x] Phase 7 — Ecosystem Hardening: ChatAgent, web dashboard, context memory, persistence, async VM

---

## What Is A2X?

A2X is a **three-layer programming language stack** for AI agents:

1. **Sigma Infinity** — the symbolic programming language / ISA. Packets are instructions, sequences of packets are programs. Uses special Unicode characters as operators for ultra-dense encoding.

2. **Omega** — the compiled latent representation. Sigma programs can be JIT-compiled into pure tensor form for maximum execution speed.

3. **CCS (CryoCore Cognitive Substrate)** — the runtime virtual machine that executes Sigma and Omega programs. Manages WorldGraph (heap), StateField (registers), and MemoryTrace (execution history).

### What Makes It a Programming Language

| Concept | A2X Equivalent |
|---------|----------------|
| Values | ConceptVectors — dense embeddings |
| Variables | Labeled nodes in WorldGraph |
| Instructions | Sigma packets (I+C+P+D fields) |
| Programs | Sequences of Sigma packets ("packet streams") |
| Control flow | Plan operators: branch, merge, loop, recursion |
| Functions | Descend into sub-plan, ascend from meta-plan |
| Memory / Heap | WorldGraph — persistent graph of concepts |
| Registers / Stack | StateField — high-dimensional working memory |
| Compilation | Sigma → Omega transformation |
| Execution | CCS runtime — Evolve, Reflect, Plan operators |

---

## Official Crates

| Crate | Purpose |
|-------|---------|
| `a2x-core` | Primitive types, traits, common enums (zero-dependency) |
| `a2x-sigma` | Sigma tokenizer, parser, packet types, SigmaProgram |
| `a2x-omega` | Omega tensor packets, compiler pipeline, decompiler |
| `a2x-bus` | Message bus, routing, transport, agent discovery |
| `a2x-ccs` | CCS VM implementation, WorldGraph, StateField, MemoryTrace |
| `a2x-agents` | Built-in agents (Orchestrator, CLI, LLM, CCS, Chat) |
| `a2x-gateway` | Entity gateway, protocol listeners, auth, entity registry |
| `a2x-client` | Rust client SDK for connecting external apps to A2X |
| `a2x-cli` | CLI binary (`a2x shell`, `a2x dashboard`, `a2x run`) |
| `a2x-probe` | Probe/debug tools for inspecting CCS internals |
| `a2x-startup` | Boot sequence, config, persistence, shutdown, key rotation |

---

## Architecture

```
┌───────────────────────────────────────────────────┐
│                    SIGMA  SOURCE                  │
│  Hyper-symbolic ISA — packets = instructions       │
│  Sequences of packets = programs                   │
├───────────────────────────────────────────────────┤
│               ↓  COMPILATION  ↓                   │
├───────────────────────────────────────────────────┤
│                    OMEGA  LATENT                  │
│  Compiled neural representation, pure tensors      │
├───────────────────────────────────────────────────┤
│               ↓  EXECUTION  ↓                     │
├───────────────────────────────────────────────────┤
│              CCS  RUNTIME / VM                    │
│  WorldGraph = heap, StateField = registers         │
│  MemoryTrace = execution log                      │
└───────────────────────────────────────────────────┘
```

---

## Where to Start Reading

- **[PLAN.md](PLAN.md)** — the full project plan and detailed design (30 sections)
- **[ROADMAP.md](ROADMAP.md)** — expansion ideas and priority order
- **[docs/](docs/)** — protocol specifications and architecture docs
- **[CHANGELOG.md](CHANGELOG.md)** — full version history and release notes

---

## Agent-to-Anything

The name says it all:
- **Agent** — any AI agent (LLM, CLI, robot, cognitive)
- **To** — the connection, the protocol, the language
- **Anything** — any other agent, any system, any environment

A2X is the language that makes "anything" reachable.

---

## Coding Standard — A2X AI-Native Coding Grade

> This standard is written for **AI agents**, not humans.

### R1: Structure & Predictability
- No hardcoded constants without named bindings
- No hidden control flow. Errors explicit via `Result<T, E>`
- Functions do one thing. Files contain one logical module.

### R2: Self-Verification
- Unit test for new functions. Integration test for new features.
- Test error variants, edge cases (empty, max, malformed).

### R3: Context Preservation
- Doc comments on all pub items
- Sub-plan references: `// See plans/03-ccs-vm.md §5`
- Rationale comments on non-obvious decisions

### R4: Determinism Boundary
- Component interfaces must be deterministic
- Learned internals may be non-deterministic
- Explicit RNG seeding in infrastructure code

### R5: Safety by Construction
- Illegal states unrepresentable via type system
- Input validation at the boundary
- `unsafe` requires justification comment

### R6: Minimal Delta
- Minimum diff required. No unrelated refactoring.
- Note issues for later, don't fix them unprompted.

### R7: Format & Conventions
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- `snake_case` functions, `CamelCase` types, `SCREAMING_SNAKE` constants
- Feature gates on optional deps

*ColdStart Intelligence Labs — Precision. Clarity. Operator-Grade.*
