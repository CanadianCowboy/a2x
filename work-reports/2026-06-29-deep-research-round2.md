# A2X Deep Research Round 2 — Protocol Patterns, Async Design & Full Pipeline Audit

> **Date:** 2026-06-29
> **Scope:** Web research on MCP/A2A/ACP/ANP + Rust async best practices + deep code audit of omega compiler, agents, and security
> **Builds on:** Round 1 work report (2026-06-29-deep-research-improvements.md)
> **Purpose:** Fills gaps in Round 1 — the omega compiler pipeline, agent implementations, security model, serialization strategy, and competitive protocol research

---

## 1. Competitive Protocol Research — Deeper Patterns

### 1.1 MCP (Model Context Protocol) — Anthropic

| Aspect | MCP Design | A2X Opportunity |
|--------|-----------|-----------------|
| **Wire format** | JSON-RPC 2.0 with typed Request/Response/Notification | A2X could expose an MCP-compatible bridge — let any LLM use A2X as an MCP tool server |
| **Initialization** | Two-phase: `initialize` request → server responds with capabilities → client sends `notifications/initialized` | A2X needs this. Currently no capability handshake between agents |
| **Capability advertisement** | Server declares `tools`, `resources`, `prompts`, `logging` support in `initialize` response | A2X should adopt an `AgentCard` concept (from A2A) with typed capabilities |
| **Streaming** | SSE for long-lived streams; resumable via `Last-Event-ID` | A2X has no streaming support. SSE would be a clean addition for long-running Σ∞ programs |
| **Session management** | `Mcp-Session-Id` header; re-initialize on 404 | A2X's gateway has no session concept — entities connect once with no state |
| **Error handling** | JSON-RPC error codes (`-32602` for version mismatch); cancellation notifications for timeouts | A2X has typed errors but no cancellation protocol |

**Key A2X takeaway:** The `initialize` handshake pattern is universal across protocols. A2X should add an `AgentHandshake` phase before program execution where agents exchange capabilities, protocol versions, and auth credentials.

### 1.2 A2A (Agent-to-Agent) — Google

| Aspect | A2A Design | A2X Opportunity |
|--------|-----------|-----------------|
| **Agent Card** | `/.well-known/agent.json` — machine-readable agent metadata (name, capabilities, endpoints, auth, modalities) | Critical missing piece. A2X agents have no structured metadata beyond string capabilities |
| **Task lifecycle** | `submitted → working → input-required → completed/failed/canceled` | A2X's scheduler has `Queued/Running/Completed/Cancelled/Failed` — close but missing `input-required` for human-in-the-loop |
| **Artifacts** | Tangible typed outputs (documents, images, datasets) | Σ∞ programs are themselves artifacts. The concept maps but isn't formalized |
| **Modality support** | Declared in Agent Card (text, video, audio, structured JSON) | A2X has `Modality` enum but it's never used in agent metadata |
| **Opaque communication** | Agents share results (artifacts), not internal state | Aligns with A2X's design — agents maintain private WorldGraphs |

**Key A2X takeaway:** The Agent Card is the single most important pattern to adopt. It should carry: typed capabilities, supported protocols (Σ∞/Ω), version, endpoint, auth method, and modality support.

### 1.3 ACP (Agent Communication Protocol) — Linux Foundation

- RESTful bridge between agent frameworks (LangChain, CrewAI, BeeAI)
- Agnostic to internal agent logic — focuses on transport interoperability
- Uses MimeTypes for multimodal data transmission
- Registry-based server architecture for agent discovery

**Key A2X takeaway:** ACP validates A2X's gateway approach. The gateway pattern (external entities ↔ A2X bus) is the right architecture. What's missing is structured entity metadata and discovery.

### 1.4 ANP (Agent Network Protocol) — Decentralized Identity

- W3C Decentralized Identifiers (DIDs) for agent authentication
- Cryptographic proofs tied to DIDs — no central CA needed
- Enables trustless cross-organizational agent interactions

**Key A2X takeaway:** Plan 12-security.md already defines Ed25519 key pairs for agent identity. This aligns with ANP's decentralized approach. Prioritize implementing agent key signing before adding higher-level features.

### 1.5 FIPA ACL — Speech Act Theory

- Messages are "performatives" (REQUEST, INFORM, QUERY, PROPOSE)
- Intent classification based on pragmatic effect, not just content
- Enables state-machine management in multi-agent workflows

**Key A2X takeaway:** A2X's IntentOp operators already map to speech acts:
- `✦` (Star/Explore) ≈ QUERY
- `⚠` (Warning) ≈ INFORM (of risk)
- `✣` (Synthesis) ≈ PROPOSE
- `✕` (Cancel) ≈ CANCEL
- `⥁` (Parallel) ≈ REQUEST (delegation)

The mapping should be formalized as a reference table in the protocol spec.

---

## 2. Rust Async Patterns — What A2X Is Missing

### 2.1 Cooperative Yielding

Tokio tasks must `await` or call `yield_now()` periodically. A2X's `AsyncRunConfig` has a `yield_every` parameter — this is correct. But the sync `CcsVm::run()` never yields, which means any sync call blocks the runtime.

**Recommendation:** The sync `run()` should remain for testing, but production code should always use `run_async()`.

### 2.2 Cancellation Tokens

`tokio_util::CancellationToken` is the standard pattern for graceful shutdown. Every spawned task should receive a token and check `token.is_cancelled()` periodically.

**Current state:** A2X has no cancellation token pattern. The scheduler's `cancel()` method calls `handle.abort()` which is forceful, not graceful.

**Recommendation:** Add `CancellationToken` to `ProgramScheduler` and pass clones to spawned VM tasks.

### 2.3 Semaphore for Capacity Limits

`tokio::sync::Semaphore` is the standard for throttling concurrent work.

**Current state:** A2X's scheduler uses a manual count of `Running` entries. This works but a `Semaphore` would be cleaner and more idiomatic.

### 2.4 Bounded Channels

`tokio::sync::mpsc::channel(capacity)` provides natural backpressure.

**Current state:** A2X uses `oneshot` channels for result delivery (correct) but `InMemoryBus` uses `mpsc::UnboundedSender` — no backpressure.

**Recommendation:** Switch bus channels to bounded with configurable capacity.

### 2.5 Status Tracking with oneshot

The standard pattern is `oneshot::Sender<Result<T, E>>` combined with `tokio::select!` for timeout handling.

**Current state:** A2X's scheduler does this correctly — it sends `Result<SigmaProgram, AgentError>` through oneshot. Round 1's CORRECT-001 claim was **false** — results ARE delivered.

---

## 3. Omega Compilation Pipeline — Deep Audit

### Pipeline Stage Status

| Stage | Plan Reference | Status | Details |
|-------|:-------------:|:------:|---------|
| **1. Lexer** | 02-omega §3 | ✅ Complete | In `a2x-sigma/tokenizer.rs`. Trie-based Unicode matching. Fuzzed. |
| **2. Parser** | 02-omega §3 | ✅ Complete | In `a2x-sigma/parser.rs`. Produces `SigmaProgram` with label table. Proptested. |
| **3. Semantic Analyzer** | 02-omega §3 | ❌ Missing | Comment says "stub — validate basic structure" but no validation code exists. Should check: jump targets, sub-program references, type consistency, contradictory operators |
| **4. IR Generator** | 02-omega §3 | ⚠️ Incomplete | `build_ir()` maps `IntentOp` → `Opcode` (10 mappings). But: (a) control flow is always empty `Vec::new()`, (b) only first intent operator is used, (c) no label resolution for jump targets, (d) no operand encoding beyond string labels |
| **5. Optimizer** | 02-omega §3 | ✅ Structure exists | All 4 passes have files. Dispatch is correct per optimization level. Need correctness audit on individual passes. |
| **6. Code Generator** | 02-omega §3 | ⚠️ Incomplete | `codegen()` iterates nodes directly (plan says topological sort). `encode_instruction()` uses deterministic Blake3 hashing — works but lossy. Ω_C (context) and Ω_D (data) regions carry hashed data the decoder can't recover. |
| **7. Serializer** | 02-omega §3 | ✅ Complete | `OmegaProgram` derives `Serialize/Deserialize`. Wire format roundtrip tests exist. |

### Critical Compiler Gaps

1. **No semantic validation whatsoever** — malformed programs with invalid jump targets compile without errors
2. **IR generator ignores control flow** — `⤈` (descend), `⤐` (branch), `⥁` (fork) are all lost during IR generation
3. **Lossy Ω encoding** — only Ω_I (intent) is recoverable via decoder. Ω_C, Ω_P, Ω_D are hashed blobs with no reverse path
4. **No topological sort in codegen** — plan specifies topological sort of dataflow graph but current impl iterates in insertion order
5. **Only first intent operator used** — a Σ∞ packet can carry multiple intent operators (e.g., `⚡✣⩫`) but only `operators[0]` is mapped

### Optimizer Pass Correctness (Needs Audit)

The 4 pass files exist but need verification:
- `constant_folding.rs` — should evaluate constant `BIND` operations at compile time
- `dead_code.rs` — should remove unused results
- `fusion.rs` — should merge adjacent instructions on same memory region
- `layout.rs` — should reorder for cache locality

---

## 4. Agent Implementation — Deep Audit

### Plan Compliance

| Agent | Plan § | Implemented | Missing |
|-------|:------:|:-----------:|---------|
| **Orchestrator** | §3 | Core dispatch works | No bus-based dispatch to other agents (runs locally only). No capability-based routing. |
| **CLI Agent** | §3, §5 | VM execution, sandbox allowlist | Container/VM sandbox modes (stubs). No actual shell command execution (only runs Σ∞ on VM). No forbidden_commands denylist. |
| **LLM Agent** | §3 | ❌ Pure stub | `nl_to_sigma()` returns empty program. `sigma_to_nl()` returns placeholder string. No LLM API integration. No Ω decompilation for inspection. |
| **CCS Agent** | §3 | Tick/cognitive loop works | `query()` is stub. No bus integration. No Ω-native execution path. |
| **Ω Agent** | §3 | ❌ Not implemented | Plan defines "Ω only — pure latent execution, max speed, zero inspectability". Doesn't exist at all. |
| **Lifecycle** | §4 | State machine exists | `Running` variant missing `vm: Box<CcsVm>` field. No heartbeat integration with bus. No disk persistence. |

### Missing Error Types

Plan §6 defines these error variants not yet implemented:

| Error Type | Status |
|-----------|:------:|
| `VmError::OutOfMemory` | ❌ Not used — WorldGraph has no allocation limit |
| `VmError::SafetyViolation` | ❌ Safety checks are counter-only stubs |
| `VmError::InvalidAddress` | ❌ Not used |
| `VmError::ParallelMergeConflict` | ❌ Not implemented |
| `VmError::MaxStepsExceeded` | ❌ Not enforced |
| `SemanticError::UndefinedLabel` | ❌ Semantic analyzer is a stub |
| `SemanticError::TypeMismatch` | ❌ Not implemented |

---

## 5. Security Model — Plan vs Reality

Plan 12-security.md defines an extensive 15-item security checklist. Current implementation status:

| # | Feature | Plan § | Status |
|:-:|---------|:------:|:------:|
| 1 | Agent Ed25519 key pair generation | §2 | ❌ Not implemented |
| 2 | Bus message signing + verification | §2 | ❌ Not implemented |
| 3 | Gateway API key authentication | §3 | ✅ Basic (InMemoryAuthProvider) |
| 4 | JWT token authentication | §3 | ❌ Stub only |
| 5 | Entity permission model | §4 | ⚠️ Struct exists but never enforced |
| 6 | CLI agent command filtering | §5 | ✅ Basic allowlist, no denylist |
| 7 | Rate limiting | §5 | ❌ Not implemented |
| 8 | Resource limits | §5 | ❌ Not implemented |
| 9 | Audit logging | §7 | ⚠️ Partial (tracing only, no SecurityEvent enum) |
| 10 | Key rotation mechanism | §6 | ❌ Not implemented |
| 11 | TLS for bus transport | §2 | ❌ Not implemented |
| 12 | TLS for gateway HTTP/WS | §3 | ❌ Not implemented |
| 13 | Secure key storage (file perms) | §6 | ❌ Not implemented |
| 14 | forbidden_commands denylist | §5 | ❌ Not implemented |
| 15 | Graceful auth failure handling | §3 | ✅ Lock errors return proper responses |

**Overall: 3/15 implemented, 2 partial, 10 not started.**

---

## 6. Serialization & Wire Format — Audit

### Per-Crate Status

| Crate | Type | Serialize? | Deserialize? | Notes |
|-------|------|:----------:|:------------:|-------|
| a2x-core | `ConceptVector` | ✅ (serde) | ✅ | Feature-gated |
| a2x-core | `RelationEdge` | ✅ (serde) | ✅ | Feature-gated |
| a2x-core | `Packet` enum | ❌ | ❌ | Round 1's PLAN-008 is correct |
| a2x-core | `SigmaPacket` | ✅ (serde) | ✅ | In a2x-sigma, not core |
| a2x-sigma | `SigmaProgram` | ❌ | ❌ | Can't serialize programs |
| a2x-omega | `OmegaPacket` | ✅ (serde) | ✅ | Const-generic tensor |
| a2x-omega | `OmegaProgram` | ✅ (serde) | ✅ | Bincode-ready |
| a2x-bus | `WireMessage` | Custom | Custom | Length-prefixed binary frames |
| a2x-bus | N/A | Bincode | — | Not used anywhere |

### Key Gap

The `Packet` enum in `a2x-core` is the unified transport type per PLAN §10 but has no `Serialize`/`Deserialize`. This means you can't serialize a Σ∞ or Ω packet over the wire using the unified type — each crate handles its own serialization. This fragments the transport layer.

---

## 7. Revised Improvement Recommendations

### Tier 1: Critical Protocol Standards (Adopt Now)

| # | What | Pattern Source | Effort |
|---|------|---------------|--------|
| **T1-1** | Add `AgentCard` struct with typed capabilities, endpoints, auth, modalities | A2A | 3-4 hours |
| **T1-2** | Add `AgentHandshake` phase — capabilities exchange before execution | MCP + A2A consensus | 4-6 hours |
| **T1-3** | Add `input-required` to `ScheduledProgramStatus` for human-in-the-loop | A2A task lifecycle | 1 hour |
| **T1-4** | Formalize IntentOp → Speech Act mapping in protocol docs | FIPA ACL | 1 hour (docs only) |

### Tier 2: Compiler Pipeline Completion

| # | What | Why | Effort |
|---|------|-----|--------|
| **T2-1** | Implement semantic analyzer (Stage 3) | No validation exists — jump targets, types, contradictions unchecked | 4-6 hours |
| **T2-2** | Wire IR generator control flow | `⤈` descend, `⤐` branch, `⥁` fork are lost during compilation | 4-6 hours |
| **T2-3** | Topological sort in codegen | Plan specifies dataflow ordering; current impl is insertion-order | 1-2 hours |
| **T2-4** | Handle multi-operator intents | `⚡✣⩫` should produce 3 IR nodes, not just map `operators[0]` | 2-3 hours |

### Tier 3: Agent Implementation Completion

| # | What | Why | Effort |
|---|------|-----|--------|
| **T3-1** | Implement LLM agent with real API integration | Core vision: "AI writes Σ∞, executes, reads result" needs this | 1-2 days |
| **T3-2** | Add bus-based dispatch to orchestrator | Orchestrator runs locally — can't coordinate other agents | 4-6 hours |
| **T3-3** | Implement `Ω Agent` | Plan defines this as a distinct agent type; doesn't exist | 2-3 hours |
| **T3-4** | Add `forbidden_commands` denylist to CLI agent | Security: blocking patterns like `rm`, `sudo`, `dd` | 1 hour |
| **T3-5** | Add `Box<CcsVm>` to lifecycle `Running` state | Plan compliance — current impl drops the VM reference | 1 hour |

### Tier 4: Security Foundation

| # | What | Why | Effort |
|---|------|-----|--------|
| **T4-1** | Add Ed25519 agent identity + message signing | Plan 12 §2; aligns with ANP decentralized identity | 1 day |
| **T4-2** | Wire permission enforcement into auth flow | `EntityPermissions` exists but never checked | 2 hours |
| **T4-3** | Add rate limiting (token bucket) | Plan 12 §5; per-entity + global limits | 2-3 hours |
| **T4-4** | Add `SecurityEvent` enum + audit logging | Plan 12 §7; structured security events | 2 hours |

### Tier 5: Async Infrastructure Hardening

| # | What | Why | Effort |
|---|------|-----|--------|
| **T5-1** | Add `CancellationToken` to scheduler | Graceful shutdown for spawned VM tasks | 2 hours |
| **T5-2** | Switch bus channels to bounded `mpsc` | Natural backpressure prevents memory exhaustion | 1 hour |
| **T5-3** | Add `Suspend`/`Resume` VM status | Enables long-running programs to checkpoint state; aligns with A2A's `input-required` | 1 day |
| **T5-4** | Fix scheduler status transitioning to `Completed` | Round 1's CORRECT-002 — status stays `Running` after completion | 1 hour |

### Tier 6: Documentation & Demo

| # | What | Why | Effort |
|---|------|-----|--------|
| **T6-1** | Create `examples/` directory with 3 runnable demos | VISION-001/002 — no working examples exist | 3-4 hours |
| **T6-2** | Add MCP bridge prototype | Strategic: lets any MCP-compatible LLM use A2X as a tool | 1-2 days |
| **T6-3** | Create `.a2x-context.md` for LLM discovery | Plan 13 — LLMs need structured metadata to use A2X | 1 hour |
| **T6-4** | Update README project status | README still says "Next: implement a2x-sigma" — 7 phases out of date | 30 min |

---

## 8. Revised Code Quality Scorecard

| Category | Old Score | New Score | Rationale |
|----------|:--------:|:---------:|-----------|
| Architecture | 8/10 | 8/10 | Layer separation still strong |
| Type Safety | 9/10 | 8/10 | `Packet` enum missing Serialize, `unwrap()` in production code |
| Test Coverage | 7/10 | 7/10 | 499 tests (not 464 as reported). Good unit coverage, no integration |
| Documentation | 5/10 | 4/10 | README stale, no examples, no mdbook, 4 sub-plans not referenced in code |
| Error Handling | 6/10 | 4/10 | 7 planned error variants never constructed. Semantic errors never produced |
| Stub Completeness | 4/10 | 3/10 | LLM agent pure stub. Ω agent doesn't exist. Semantic analyzer missing |
| Safety | 3/10 | 3/10 | No change — counter-only stubs |
| Concurrency | 5/10 | 5/10 | No change — scheduler works but no CancellationToken, unbounded channels |
| Security | N/A | 2/10 | New category. 3/15 plan items implemented |
| Compiler Completeness | N/A | 4/10 | New category. 3/7 stages complete, 3 partial, 1 missing |

**Overall: 4.8/10** (down from 6.0 — the deeper audit reveals more gaps than Round 1 found)

---

## 9. Recommended Priority Order

### This Week
1. Fix README project status (T6-4) — 30 min, high visibility
2. Add `AgentCard` struct (T1-1) — aligns with industry standard
3. Add `forbidden_commands` denylist (T3-4) — security quick win
4. Wire permission enforcement (T4-2) — security quick win
5. Fix scheduler status transitions (T5-4) — Round 1 bug fix
6. Create `examples/` directory (T6-1) — makes vision tangible

### Next 2 Weeks
7. Implement semantic analyzer (T2-1) — critical compiler gap
8. Add `input-required` status + `Suspend`/`Resume` (T1-3 + T5-3)
9. Add bus-based dispatch to orchestrator (T3-2)
10. Add rate limiting (T4-3)
11. Add `CancellationToken` to scheduler (T5-1)

### Next Month
12. Implement LLM agent with real API (T3-1) — unlocks the core vision
13. Wire IR generator control flow (T2-2)
14. Add Ed25519 agent identity (T4-1)
15. Handle multi-operator intents in compiler (T2-4)
16. MCP bridge prototype (T6-2) — strategic ecosystem play

### Next Quarter
17. Security hardening — TLS, key rotation, audit logging (remaining 10 plan items)
18. Ω Agent implementation (T3-3)
19. Python/JS client SDKs
20. Web dashboard (WASM)

---

## 10. Protocol Positioning — A2X's Unique Niche

After deep research, A2X's differentiation is clearer:

| Protocol | Layer | A2X Relationship |
|----------|-------|------------------|
| **MCP** | LLM ↔ Tool | A2X can BE an MCP server — expose Σ∞ execution as MCP tools |
| **A2A** | Agent ↔ Agent | A2X is a PROGRAMMING LANGUAGE for agents, not a messaging protocol. A2A agents send tasks; A2X agents send programs. |
| **ACP** | Framework ↔ Framework | A2X's gateway serves the same bridging role for entities |
| **ANP** | Identity | A2X's planned Ed25519 identity aligns; implement it |

> **A2X's unique value: "The only system where AI agents write, compile, and execute programs in a language designed for machine cognition — with a differentiable runtime that learns from execution."**

The three differentiators:
1. **It's a language** (not a protocol, not a framework)
2. **It's for AI** (Unicode operators designed for machine density, not human readability)
3. **It's learnable** (the whole stack is differentiable — operators can be trained)

---

## 11. Round 1 Report Corrections

| Round 1 Claim | Correction |
|---------------|-----------|
| CORRECT-001: "Scheduler never delivers results" | **FALSE.** Line 128 of `scheduler.rs`: `let _ = result_tx.send(result)` — results ARE delivered |
| "~464 tests" | Actual count: **499** `#[test]` functions |
| ARCH-001: "std::sync::Mutex blocks async handlers" | **Overstated.** HTTP handlers use `.lock().map_err()`, not `.unwrap()`. Impact minimal until real tokio server is spawned |
| "Phase 7 async scaffolding" | No "Phase 7" in PLAN.md. Correct reference is `plans/10-concurrency.md` |
| 32 stubs total | Count not independently verified but seems reasonable |

---

*Web research sources:*
- MCP spec: modelcontextprotocol.io/specification/2025-11-25
- A2A: developers.googleblog.com — A2A announcement; galileo.ai; mindstudio.ai
- ACP: agentcommunicationprotocol.dev
- ANP: agent-network-protocol.com
- Tokio patterns: tokio.rs/blog, docs.rs/tokio
- FIPA ACL: fipa.org, towardsai.net
