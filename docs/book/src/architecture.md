# Architecture Overview

A2X is a layered system. Each layer transforms and transports cognitive
programs from human-readable Σ∞ to executable Ω tensors on the CCS virtual
machine.

## Crate Map

```
┌─────────────────────────────────────────────────┐
│  a2x-cli       a2x-gateway      a2x-client      │  ← User-facing
├─────────────────────────────────────────────────┤
│  a2x-agents    a2x-startup      a2x-probe       │  ← Runtime services
├─────────────────────────────────────────────────┤
│  a2x-omega     a2x-bus          a2x-ccs         │  ← Core engine
├─────────────────────────────────────────────────┤
│  a2x-sigma     a2x-core                          │  ← Foundation
└─────────────────────────────────────────────────┘
```

## Layer Descriptions

### Foundation Layer

| Crate | Purpose |
|-------|---------|
| `a2x-core` | Types shared by all crates: `ConceptVector`, `WorldGraph`, `StateField`, `Agent`, `NodeId`, `RelationType` |
| `a2x-sigma` | Σ∞ protocol: tokenizer, parser, `SigmaProgram`, `SigmaPacket`, binary encoding |

### Core Engine

| Crate | Purpose |
|-------|---------|
| `a2x-omega` | Ω compiler: Σ∞ → tensor IR → optimized Ω packets, 7-stage pipeline |
| `a2x-ccs` | CCS VM: WorldGraph (petgraph), StateField (ndarray), 7 operators, probe API |
| `a2x-bus` | Agent communication: discovery, routing, TCP transport, TLS, identity verification |

### Runtime Services

| Crate | Purpose |
|-------|---------|
| `a2x-agents` | Agent implementations: ChatAgent (Ollama/OpenAI), Orchestrator, CCS agent |
| `a2x-probe` | Debugging: breakpoints, instruction tracer, memory timeline |
| `a2x-startup` | Boot/shutdown: config loading, persistence, key rotation, graceful shutdown |

### User-Facing

| Crate | Purpose |
|-------|---------|
| `a2x-cli` | Command-line interface: shell, dashboard, monitor, parse, run, agents, probe |
| `a2x-gateway` | Entity gateway: HTTP/WS/TCP/stdio listeners, auth, rate limiting, web dashboard |
| `a2x-client` | Rust SDK for external applications connecting to the gateway |

## Data Flow

```
User types Σ∞ text
    ↓
a2x-sigma: tokenize → parse → SigmaProgram
    ↓
a2x-omega: semantic analysis → IR → optimize → Ω tensor
    ↓
a2x-ccs: load → fetch → decode → execute (7 operators)
    ↓
a2x-core: WorldGraph updated, StateField modified, MemoryTrace recorded
    ↓
a2x-probe: breakpoints hit, traces collected
a2x-gateway: dashboard updated via WebSocket
```

## Multi-Agent Communication

Agents communicate through the **Bus** — a publish-subscribe transport layer:

- **Discovery:** Agents register capabilities and types
- **Routing:** Messages delivered by agent ID, type, or capability
- **Transport:** TCP, TLS, or in-process channels
- **Identity:** Ed25519 signing for agent authentication
