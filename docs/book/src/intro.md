# A2X — Agent-to-Anything

**AI-native programming language and cognitive runtime.**

A2X lets you program AI agents using Σ∞ (Sigma-Infinity) — a Unicode-based
protocol language designed for cognitive operations. Agents express ideas
through operators like **Bind** (synthesize concepts), **Differentiate**
(split ideas), **Ground** (anchor to reality), and **Evolve** (learn over time).

Programs run on the **CCS** (Cognitive Control Substrate), a virtual machine
with a WorldGraph of concepts, a StateField of beliefs, and a MemoryTrace of
experience.

## Why A2X?

| Traditional PL | A2X |
|---------------|-----|
| Variables & functions | Concepts & relations |
| Control flow | Plan operators (branch, swarm, recurse) |
| Compiler IR | Latent tensor representation (Ω) |
| Debugger | Probe with breakpoint + tracer |
| Single process | Multi-agent bus with discovery |

## Architecture at a Glance

```
┌──────────────┐    Σ∞     ┌──────────┐    Ω      ┌──────────┐
│  a2x CLI     │ ────────→ │  Omega   │ ────────→ │   CCS    │
│  Dashboard   │           │ Compiler │           │    VM    │
└──────────────┘           └──────────┘           └──────────┘
       │                                                 │
       └──────────── Bus (agent discovery) ──────────────┘
                                │
                    ┌───────────┴───────────┐
                    │    Gateway + Agents    │
                    │  ChatAgent, Orchestrator│
                    └───────────────────────┘
```

## Current Status

**v0.9.0-alpha** — 12 crates, 70+ tests, workspace compiles with zero warnings.

- ✅ Σ∞ Protocol Core — tokenizer, parser, all operator tables
- ✅ CCS Cognitive Substrate — WorldGraph, StateField, 7 operators, VM loop
- ✅ Ω Latent Protocol — 7-stage compiler with optimizer
- ✅ Bus — agent discovery, routing, TCP transport, TLS
- ✅ Agents — ChatAgent (Ollama/OpenAI), Orchestrator, CCS agent
- ✅ Web Dashboard — live agents, WorldGraph graph, heatmaps, Chat tab
- ✅ CLI — shell, dashboard, monitor, parse, run, agents, probe
- ✅ Gateway — HTTP/WS/TCP/stdio listeners, entity auth, rate limiting

[Get started →](quick-start.md)
