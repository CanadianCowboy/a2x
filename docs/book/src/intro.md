# ⚡ A2X — Agent-to-Anything

<p align="center">
  <strong>An AI-native programming language and cognitive runtime.</strong><br>
  <em>Sigma ISA → Omega Compiler → CCS Virtual Machine</em>
</p>

<p align="center">
  <a href="https://github.com/CanadianCowboy/a2x">GitHub</a> ·
  <a href="https://github.com/CanadianCowboy/a2x/releases/tag/v0.9.0-alpha">v0.9.0-alpha</a> ·
  <a href="https://github.com/CanadianCowboy/a2x/discussions/1">Discussions</a> ·
  <a href="https://github.com/CanadianCowboy/a2x/blob/master/LICENSE">AGPL-3.0</a>
</p>

---

**A2X is not a traditional programming language.** It has no keywords, no human-readable syntax. It is a three-layer stack that AI agents use to write, compile, and execute programs at machine speed. A single Sigma packet encodes what would take hundreds of LLM tokens.

This is the documentation. Read it to understand the stack, then [get started](quick-start.md).

---

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
