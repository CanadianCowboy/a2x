# A2X v0.9.0-alpha — "The Visible Mind"

> **Released:** 2026-07-09  
> **Tag:** `v0.9.0-alpha`  
> **License:** AGPL-3.0  
> **Rust MSRV:** 1.75+

---

## The Release

**v0.9.0-alpha is not an incremental update. It is the first release where A2X becomes visible.**

For seven phases, A2X was a protocol. A spec. A collection of crates that compiled but had no face. That changes today. You can now open a browser at `localhost:8778` and watch the WorldGraph evolve in real-time. You can type a Sigma program and see it execute. You can talk to an AI agent that reasons about the ecosystem using the CCS VM as its cognitive substrate.

This is the moment A2X stops being a protocol spec and starts being a product.

---

## What's New

### 🌐 Web Dashboard

The centerpiece of this release. A single HTML file served by the gateway with WebSocket streaming — no build tooling, no npm.

- **Agent cards** — live status, capabilities, entity type badges
- **WorldGraph** — force-directed graph of concept nodes and relation edges, fed from the live CCS VM
- **StateField heatmap** — tensor region visualization with color-coded tiles
- **VM inspector** — status, region readout, memory trace
- **Sigma playground** — write and execute programs inline, see results immediately
- **Bus traffic log** — real-time event stream from the message bus
- **Context usage bar** — tokens used / remaining in chat mode

```bash
cargo run --release -p a2x-cli -- dashboard
# Open http://127.0.0.1:8778
```

### 🤖 Chat Agent

A conversational AI agent with tool execution. Connect to Ollama or OpenAI and talk to an agent that can:

- Execute Sigma programs
- Inspect the CCS VM state
- Query the bus for agent discovery
- Run CLI commands (sandboxed)
- Access Omega compiler diagnostics
- Persist conversations to `~/.a2x/conversations/`

```bash
A2X_CHAT_BACKEND=ollama A2X_CHAT_MODEL=llama3.2 cargo run --release -p a2x-cli -- dashboard
```

Built with: sliding window context pruning, context memory (Copilot-style mined file paths and topics), 14 tool definitions, and streaming responses via WebSocket.

### 🖥️ CLI Shell

An interactive Sigma REPL with colored output:

```bash
cargo run --release -p a2x-cli -- shell
```

Features: command history, syntax highlighting, inline execution results.

### 🧠 WorldGraph Bootstrap

The CCS VM now boots with a pre-loaded knowledge graph: 12 concept nodes and 10 relation edges. Agents have a baseline world-model from the moment they start.

### 🔒 Bus Hardening

- Duplicate agent detection and lifecycle management
- Online/offline state tracking
- Discovery deduplication
- BusBridge convenience wrapper for domain events

### ⚡ CCS VM Fork/Merge

Parallel swarm execution: the VM can fork child VMs with snapshotted state, run them in parallel, and merge results back into the parent WorldGraph.

### 📦 Client SDKs

Rust (`a2x-client`), Python, and TypeScript SDKs for connecting external applications to A2X.

---

## Architecture at v0.9

```
┌───────────────────────────────────────────────────┐
│                  SIGMA SOURCE                      │
│  40+ Unicode operators, packet-stream programs     │
├───────────────────────────────────────────────────┤
│               ↓  COMPILATION  ↓                   │
├───────────────────────────────────────────────────┤
│                  OMEGA LATENT                      │
│  7-stage compiler pipeline, 4 optimizer passes     │
├───────────────────────────────────────────────────┤
│               ↓  EXECUTION  ↓                     │
├───────────────────────────────────────────────────┤
│                CCS RUNTIME / VM                    │
│  WorldGraph + StateField + MemoryTrace             │
│  Fork/Merge, 7 operators, parallel swarms          │
└───────────────────────────────────────────────────┘
```

---

## By the Numbers

| Metric | Value |
|--------|-------|
| Crates | 12 |
| Tests | 70+ passing |
| Clippy warnings | Zero (`-D warnings`) |
| Lines of Rust | 25,000+ |
| Sub-plans | 15 design documents |
| Doc pages | 35 (mdBook) |
| CI platforms | Ubuntu + Windows |

---

## What's Next

v0.9.0-alpha is a milestone, not a destination. On the roadmap:

- **WASM runtime** — execute A2X programs in the browser
- **Real Python SDK** — `pip install a2x-client`
- **Benchmark suite** — prove performance with Criterion
- **Learned compiler** — neural encoder/decoder for Omega
- **Multi-machine swarms** — agents across multiple hosts

See [ROADMAP.md](ROADMAP.md) for the full expansion plan.

---

## Getting Started

```bash
git clone https://github.com/CanadianCowboy/a2x.git
cd a2x
cargo build --release
cargo run --release -p a2x-cli -- dashboard
```

---

## License

**AGPL-3.0** — full copyleft. Forks stay open. Cloud wrappers stay open. The end user is always protected.

---

<p align="center">
  <strong>ColdStart Intelligence Labs</strong><br>
  <em>Precision. Clarity. Operator-Grade.</em><br>
  <em>A2X — Agent to Anything.</em>
</p>
