# A2X — Sub-Plans Index

This directory contains focused sub-plans for each major area of the A2X project. These are extracted from the master PLAN.md and maintained as standalone documents.

> **Start here:** [PLAN.md](../PLAN.md) for the high-level overview, architecture, and design principles.
> **Then dive into** whichever sub-plan matches what you're implementing.

---

## Sub-Plans (in dependency order)

| # | Sub-Plan | Covers | Dependencies |
|:-:|----------|--------|:------------:|
| 01 | [Sigma Language](01-sigma-language.md) | Σ∞ ISA, instruction format, operator tables, tokenizer, parser, SigmaProgram, binary encoding | None |
| 02 | [Ω Compiler](02-omega-compiler.md) | Ω packet shape, compilation pipeline (7 stages), IR, optimizer passes, encoder/decoder | 01 |
| 03 | [CCS VM](03-ccs-vm.md) | CCS runtime, fetch-decode-execute loop, WorldGraph, StateField, MemoryTrace, ISA opcode, addressing modes | 01, 02 |
| 04 | [Bus Protocol](04-bus.md) | Message bus, transport trait, wire format, agent discovery, routing, connection lifecycle | 01 |
| 05 | [Agents](05-agents.md) | Agent trait, built-in types, lifecycle, safety model, error types, sandboxing | 01, 03, 04 |
| 06 | [Entity Gateway](06-entity-gateway.md) | Entity integration, gateway service, protocol listeners, auth, client SDKs | 04, 05 |
| 07 | [Probe & Debug](07-probe.md) | Probe protocol, breakpoints, single-stepping, channel separation, perf impact, probe CLI | 03 |
| 08 | [Ecosystem](08-ecosystem.md) | Crate structure, feature gating, Git workflow, testing, CI/CD, versioning, contribution model | All |
| 09 | [Core Types](09-core-types.md) | a2x-core: ConceptVector, RelationEdge, WorldGraph trait, StateField trait, NodeId, ProgramId, error types | None |
| 10 | [Concurrency](10-concurrency.md) | Async model, multi-program scheduling, parallel swarm (⥁) internals, thread safety | 01, 03 |
| 11 | [Startup & Shutdown](11-startup-shutdown.md) | Boot order, config loading, state persistence, crash recovery, graceful shutdown, directory layout | All |
| 12 | [Security](12-security.md) | Auth methods, bus encryption, entity permissions, CLI sandboxing, rate limiting, key rotation, audit logging | 04, 05, 06 |
| 13 | [Documentation](13-documentation.md) | Doc tools (rustdoc, mdbook), document inventory, generation CI, standards, AI context | All |
| 14 | [Resilience](14-resilience.md) | Graceful degradation, fault tolerance, crash recovery, storage corruption, resource exhaustion, watchdog | 03, 04, 05 |
| 15 | [WASM Support](15-wasm.md) | Browser-based agents, WASM CCS VM, IndexedDB storage, WebSocket transport, web dashboards | 01, 03, 04 |

---

## Quick Legend

- **Σ∞** = Sigma Infinity — the symbolic programming language / ISA
- **Ω** = Omega — compiled latent tensor representation
- **CCS** = CryoCore Cognitive Substrate — the runtime VM
- **WorldGraph** = graph-structured heap (persistent memory)
- **StateField** = high-dimensional tensor registers (working memory)
- **MemoryTrace** = execution history (program counter log)
- **PolicyField** = JIT compiler + optimizer
- **Entity** = any external system/user connected through the gateway
