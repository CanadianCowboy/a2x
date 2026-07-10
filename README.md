<p align="center">
  <h1 align="center">⚡ A2X — Agent-to-Anything</h1>
  <p align="center"><em>An AI-native programming language and runtime — and an experiment in a new standard for how open-source is built when AI is in the loop.</em></p>
  <p align="center">
    <a href="https://github.com/CanadianCowboy/a2x/blob/master/LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-blue.svg" alt="License: AGPL-3.0"></a>
    <a href="https://github.com/CanadianCowboy/a2x/releases"><img src="https://img.shields.io/badge/version-v0.9.0--alpha-orange.svg" alt="Version"></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.75%2B-brightgreen.svg" alt="Rust: 1.75+"></a>
    <a href="https://github.com/CanadianCowboy/a2x/actions"><img src="https://github.com/CanadianCowboy/a2x/actions/workflows/ci.yml/badge.svg" alt="CI Status"></a>
  </p>
</p>

---

## Contents

- [What Is A2X?](#what-is-a2x)
- [Quick Start](#quick-start)
- [Features](#features)
- [Architecture](#architecture)
- [Crates](#crates)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

---

## What Is A2X?

A2X is a **programming language built for AI agents**, not humans. It has no keywords, no syntax in the traditional sense — instead, it's a three-layer stack that agents use to **write, compile, and execute programs** at machine speed.

```
Σ∞ (Sigma)         →    Ω (Omega)         →    CCS (Runtime)
Symbolic ISA            Compiled tensors        Cognitive VM
"Source code"           "Machine code"          "The CPU"
```

### The Stack

| Layer | Role | Analogy |
|-------|------|---------|
| **Σ∞** Sigma Infinity | Symbolic programming language / ISA. Dense Unicode instructions. | Assembly language |
| **Ω** Omega | Compiled latent representation. Pure tensors, zero symbols. | Compiled binary |
| **CCS** CryoCore Cognitive Substrate | Runtime VM. Executes programs, manages memory and state. | OS + CPU |

### Why This Exists

LLMs think in vectors but are forced to communicate in words. A2X removes the bottleneck — one Sigma packet encodes what would take hundreds of tokens. Agents don't just message each other; they **program each other**.

### A New Standard

A2X is two things: a language, and an operating model. The way this project is built — with AI agents as first-class contributors, mandatory work reports, enforceable code standards, and radical transparency — is as much the product as the compiler. Read [CONTRIBUTING.md](CONTRIBUTING.md) to understand the standard.

> **The direction of this project is not fully determined.** If something here excites you, pursue it. You don't need permission. Fork it, build it, show us.

---

## Project Status

**Alpha. Active exploration. Direction not fixed.**

A2X is real, functional, and tested — 70+ tests, 12 crates, web dashboard, chat agent, CCS VM. But it is not a finished product and it does not have a fixed roadmap. The plans in [ROADMAP.md](ROADMAP.md) are possibilities, not promises.

What becomes real depends on who shows up and what excites them. If you see something here you want to build, that direction is as valid as any other. **Open a PR, start a discussion, or just fork it and go.**

### What's Solid

| Area | Status |
|------|--------|
| Sigma tokenizer + parser | Stable, fuzz-tested, property-tested |
| CCS VM (7 operators, Fork/Merge) | Functional, integration-tested |
| Omega compiler pipeline | Functional, wire-format tested |
| Message bus + discovery | Functional, cross-machine tested |
| Web dashboard | Live, WebSocket streaming |
| Chat agent (Ollama/OpenAI) | Functional, 14 tools |
| CLI (shell, dashboard, monitor) | Functional |
| Client SDKs (Rust, Python, TS) | Functional |

### What's Open

See [ROADMAP.md](ROADMAP.md) for the full list. Highlights:

- **WASM runtime** — execute A2X in browsers
- **Python SDK on PyPI** — `pip install a2x-client`
- **Benchmark suite** — prove performance at scale
- **Learned compiler** — neural encoder/decoder for Omega
- **Multi-machine swarms** — agents across hosts
- **Community contributions** — whatever you bring

---

## Quick Start

```bash
# Clone
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x

# Build
cargo build --release

# Launch the dashboard (browser opens at http://127.0.0.1:8778)
cargo run --release -p a2x-cli -- dashboard

# With Ollama chat enabled
A2X_CHAT_BACKEND=ollama A2X_CHAT_MODEL=llama3.2 cargo run --release -p a2x-cli -- dashboard

# Interactive Sigma REPL
cargo run --release -p a2x-cli -- shell

# Execute a Sigma program
cargo run --release -p a2x-cli -- run --program "⟦Σ∞⟧⟬I:✕ ∷ P:✕⟭"
```

### CLI Commands

| Command | What it does |
|---------|-------------|
| `a2x dashboard` | Web dashboard with live agent graph, VM inspector, and chat |
| `a2x shell` | Interactive Sigma REPL with colored output |
| `a2x monitor` | Live bus traffic viewer |
| `a2x run -p <expr>` | Parse and execute a Sigma program |
| `a2x parse -p <expr>` | Parse and display a Sigma program |
| `a2x agents` | List registered agents with capabilities |
| `a2x probe <id>` | Inspect an agent's internal state |

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `A2X_CHAT_BACKEND` | `none` | `ollama` or `openai` for the chat agent |
| `A2X_CHAT_MODEL` | `llama3.2` | Model name |
| `A2X_HTTP_PORT` | `8778` | Dashboard and API port |
| `A2X_API_KEY` | (none) | API key for authenticated HTTP access |

---

## Features

### 🌐 Web Dashboard
Live visualization of the entire system in your browser — agent cards, force-directed concept graph, tensor heatmaps, VM inspector, Sigma playground, and streaming chat with model switching.

### 🤖 Built-in Chat Agent
Connect to Ollama or OpenAI and talk to an AI that can execute A2X programs and reason about the ecosystem in real-time.

### 📡 Multi-Protocol Gateway
HTTP REST API, WebSocket streaming, TCP binary, and stdin/stdout — anything that can make a network connection can speak A2X.

### 🧠 CCS Virtual Machine
A full cognitive runtime: WorldGraph (heap), StateField (registers), MemoryTrace (execution history), 7 core operators, and a parallel swarm execution model.

### 🔧 Sigma Programming Language
40+ Unicode operators across Intent, Context, Plan, and Data categories. Programs are sequences of packets. Supports branching, sub-plans, recursion, parallel forks, and self-modifying code.

### 📊 Omega Compiler
JIT compilation pipeline from Sigma to Omega with optimizer passes: constant folding, dead code elimination, instruction fusion, and layout optimization.

### 📦 Client SDKs
Connect external applications to A2X with Rust, Python, or TypeScript SDKs.

### 🔒 AGPL-3.0 Licensed
Full copyleft — forks, modifications, and cloud deployments all must stay open.

---

## Architecture

```
┌───────────────────────────────────────────────────┐
│                  SIGMA SOURCE                      │
│  Hyper-symbolic ISA — packets are instructions     │
│  Sequences of packets form programs                │
├───────────────────────────────────────────────────┤
│               ↓  COMPILATION  ↓                   │
├───────────────────────────────────────────────────┤
│                  OMEGA LATENT                      │
│  Compiled neural representation, pure tensors      │
├───────────────────────────────────────────────────┤
│               ↓  EXECUTION  ↓                     │
├───────────────────────────────────────────────────┤
│                CCS RUNTIME / VM                    │
│  WorldGraph = heap, StateField = registers         │
│  MemoryTrace = execution log                       │
└───────────────────────────────────────────────────┘
```

---

## Crates

| Crate | Purpose | Dependencies |
|-------|---------|:---:|
| `a2x-core` | Primitives, traits, enums | Zero |
| `a2x-sigma` | Tokenizer, parser, packet types | core |
| `a2x-omega` | Tensor packets, compiler pipeline | core |
| `a2x-bus` | Message bus, routing, transport, discovery | core, sigma |
| `a2x-ccs` | VM, WorldGraph, StateField, MemoryTrace | core |
| `a2x-agents` | Orchestrator, CLI, LLM, CCS, Chat agents | core, sigma, bus, ccs |
| `a2x-gateway` | Entity gateway, HTTP/WS/TCP/stdio listeners | bus, sigma |
| `a2x-client` | Rust client SDK | core |
| `a2x-cli` | CLI binary (`a2x` command) | agents, bus |
| `a2x-probe` | Debug tools, tracer, inspector | ccs |
| `a2x-startup` | Boot sequence, config, persistence, shutdown | core |

---

## Documentation

- **[ROADMAP.md](ROADMAP.md)** — Expansion plans and priorities
- **[PLAN.md](PLAN.md)** — Full 30-section design document
- **[CHANGELOG.md](CHANGELOG.md)** — Version history
- **[docs/](docs/)** — Protocol specifications and architecture deep-dives
- **[CONTRIBUTING.md](CONTRIBUTING.md)** — How to contribute

---

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development

```bash
# Run all tests
cargo test --workspace --lib

# Check formatting + lints
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings

# Pre-commit hook
bash scripts/setup-hooks.sh
```

### CI/CD

GitHub Actions runs on every push to `master`:
- `cargo fmt --check`
- `cargo clippy -D warnings`
- `cargo build`
- `cargo test`
- Ubuntu + Windows

---

## License

**[AGPL-3.0](LICENSE)** — GNU Affero General Public License v3.0.

This is a strong copyleft license that requires anyone who distributes or serves this software (including as a cloud service) to share their source code. Forks must stay open. Cloud wrappers must stay open. The end user is always protected.

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong><br>
  <em>Precision. Clarity. Operator-Grade.</em>
</p>

