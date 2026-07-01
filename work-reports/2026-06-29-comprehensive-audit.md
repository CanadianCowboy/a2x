# A2X Comprehensive Audit & Improvement Roadmap

> **Date:** 2026-06-29
> **Scope:** Full codebase audit (all 10 crates) + web research on MCP/A2A/ACP/ANP/FIPA ACL + deep pipeline analysis + plan compliance matrix
> **Builds on:** Round 1 work report (2026-06-29-deep-research-improvements.md) — findings incorporated, errors corrected
> **Baseline:** v0.6.0 + uncommitted Phase 7 (concurrency) scaffolding
> **Purpose:** Single source of truth — all findings, all recommendations, all corrections, no omissions

---

## Executive Summary

A2X is a remarkably ambitious project with a solid architecture foundation. Across **10 crates**, **499 tests**, and **15 sub-plans**, the codebase demonstrates careful layer separation (Σ∞→Ω→CCS→Bus→Agents→Gateway) and strong type safety. However, a deep audit reveals significant gaps between plan and implementation, particularly in the compiler pipeline, agent completeness, security model, and documentation.

**Overall Code Quality Score: 4.8/10** — Strong architecture undermined by deep gaps in core subsystems.

### Top 5 Findings

1. **The Omega compiler has 3 incomplete stages** — semantic analysis is missing entirely, IR generator ignores control flow, codegen skips topological sort
2. **LLM agent is a pure stub** — the core vision ("AI writes Σ∞ → CCS executes → AI reads result") can't be demonstrated
3. **Security is 3/15 plan items implemented** — entity permissions stored but never enforced, no message signing, no rate limiting
4. **No working examples directory** — a project claiming to be "a language for AI" can't show it doing anything
5. **The README is 7 phases out of date** — still says "Next: implement a2x-sigma" when 7 phases have been completed since

---

## Part A: Phase-Level Progress Assessment

PLAN.md §18 defines 6 phases. Here is where the project actually stands:

| Phase | Name | Status | Key Deliverables |
|:-----:|------|:------:|------------------|
| **0** | Scaffold & Core | ✅ Complete | Workspace, all 10 crates, a2x-core fully implemented |
| **1** | Σ∞ Protocol Core | ✅ Complete | Tokenizer, parser, operator tables, SigmaProgram, fuzz tests |
| **2** | CCS Cognitive Substrate | ✅ Complete | WorldGraph, StateField, MemoryTrace, all 7 operators (basic), VM loop |
| **3** | Ω Latent Protocol | ⚠️ Partial | OmegaPacket exists, compiler 3/7 stages complete, decoder recovers intent only |
| **4** | Training & Learning | ✅ Complete (basic) | Learned encoder/decoder stubs, simulated environment, training loop stub |
| **5** | Probe & Debug | ✅ Complete | Probe protocol, breakpoints, tracer, inspector, probe CLI |
| **6** | Entity Gateway | ⚠️ Partial | Gateway struct exists, 4 protocol listeners, auth stub. Missing: `add_listener()`, permission enforcement |
| **7** | Concurrency | 🔄 In Progress | Async bus, async VM, parallel swarm, scheduler — scaffolded, test failures |

**Phase 7 does not exist in PLAN.md** — the correct reference is `plans/10-concurrency.md`.

---

## Part B: Bug Inventory — Verified

### Confirmed Critical Bugs

| ID | Severity | Crate | Description |
|----|:--------:|-------|-------------|
| **BUG-001** | 🔴 | a2x-gateway | `GatewayState.listeners` is always empty — no `add_listener()` method. Gateway is structurally complete but functionally inert |
| **BUG-002** | 🔴 | a2x-ccs | Scheduler status never transitions to `Completed` — stays `Running` after task finishes |
| **BUG-003** | 🟡 | a2x-bus | `tcp_transport.rs::decode_message()` has 4 `.try_into().unwrap()` calls that panic on malformed network frames |
| **BUG-004** | 🟡 | a2x-gateway | `Gateway::register_builtin_agents()` uses `.lock().unwrap()` — mutex poisoning panics gateway |
| **BUG-005** | 🟡 | a2x-gateway | `EntityPermissions` stored in auth but never checked — rate limits, instruction caps, probe access are dead config |
| **BUG-006** | 🟡 | a2x-gateway | `#[allow(dead_code)]` on `HttpListener::router()`, `incoming_tx`/`response_rx` in WS/TCP listeners, `AuthQuery` struct |
| **BUG-007** | 🟢 | a2x-probe | `tracer.rs::run()` logs `Opcode::Nop` after step (opcode consumed). `run_probed()` is correct |

### Round 1 Bug Corrigenda

| Round 1 Claim | Verdict | Reality |
|---------------|:-------:|---------|
| CORRECT-001: "Scheduler never delivers results" | ❌ **FALSE** | Line 128 of `scheduler.rs`: `let _ = result_tx.send(result)` — results ARE delivered via oneshot |
| ARCH-001: "std::sync::Mutex blocks async HTTP handlers" | ⚠️ **OVERSTATED** | Handlers use `.lock().map_err()` not `.unwrap()`. `HttpListener::start()` is a stub (sets a bool, never spawns tokio server). Valid concern but minimal current impact |
| "~464 tests" | 📝 | Actual count is **499** `#[test]` functions |
| "Phase 7 async scaffolding" | 📝 | No "Phase 7" in PLAN.md. Correct reference: `plans/10-concurrency.md` |

---

## Part C: Full Stub Inventory (47 total across all crates)

### a2x-ccs (14 stubs)

| File | Stub | Nature |
|------|------|--------|
| `operators/bind.rs` | `bind()` averages vectors element-wise | Basic impl — no neural merging |
| `operators/differentiate.rs` | `differentiate()` splits vector into contiguous chunks | Basic impl — no semantic splitting |
| `operators/ground.rs` | `ground()` wraps raw data directly into ConceptVector | Basic impl — no modality processing |
| `operators/evolve.rs` | `evolve()` time-steps with simple operations | Basic impl |
| `operators/reflect.rs` | `reflect()` allocates self-model node | Basic impl |
| `operators/plan.rs` | `plan()` generates simple action list | Basic impl |
| `operators/actuate.rs` | `actuate()` returns no-op command | Stub — never issues real commands |
| `policy.rs` | `UniformPolicy` returns uniform distribution | Stub — no learned policy |
| `safety.rs` | `SafetyLevel::Bounded` is counter-only — no memory/capability enforcement | Critical stub |
| `world_graph.rs` | `GraphQuery::Custom` returns `Vec::new()` | Stub |
| `world_graph.rs` | `GraphQuery::BySimilarity` unimplemented | Implicit stub |
| `memory.rs` | Compression is identity pass | Stub |
| `probe.rs` | Breakpoint system works but tracing incomplete (see BUG-007) | Minor |
| `scheduler.rs` | Status never transitions to Completed (see BUG-002) | Bug |

### a2x-agents (7 stubs)

| File | Stub | Nature |
|------|------|--------|
| `llm_agent.rs` | `nl_to_sigma()` returns empty program | Critical stub — core vision depends on this |
| `llm_agent.rs` | `sigma_to_nl()` returns placeholder string | Critical stub |
| `llm_agent.rs` | `execute()` returns empty Packet | Stub |
| `ccs_agent.rs` | `start_cognitive_loop()` marks bool, no background thread | Stub |
| `ccs_agent.rs` | `query()` returns empty program | Stub |
| `orchestrator.rs` | `dispatch()` runs locally — no bus-based dispatch to other agents | Missing feature |
| `lifecycle.rs` | `AgentState::Running` missing `vm: Box<CcsVm>` field per plan | Plan deviation |

### a2x-omega (5 stubs/gaps)

| File | Stub | Nature |
|------|------|--------|
| `compiler.rs` | Stage 3 semantic analyzer: "stub — validate basic structure" with no validation code | Critical gap |
| `compiler.rs` | `build_ir()` — control flow always `Vec::new()`, only first intent operator used | Major gap |
| `compiler.rs` | `codegen()` — no topological sort (plan specifies dataflow ordering) | Gap |
| `decoder.rs` | Only Ω_I (intent) recoverable — Ω_C/Ω_P/Ω_D are hashed blobs with no reverse path | Architectural gap |
| `encoder.rs` | File does not exist — learned encoder is in `learned_encoder.rs`, basic encoder is inline in compiler | Plan deviation |

### a2x-gateway (4 stubs/gaps)

| File | Stub | Nature |
|------|------|--------|
| `auth.rs` | JWT validation stub: "accept any non-empty token" | Security stub |
| `gateway.rs` | No `add_listener()` — listeners Vec always empty | Critical gap (BUG-001) |
| `gateway.rs` | `EntityPermissions` never enforced | Security gap (BUG-005) |
| `listeners/http.rs` | `HttpListener::start()` marks bool, never spawns tokio server | Stub |

### a2x-sigma (2 implicit stubs)

| Area | Status |
|------|--------|
| Binary ISA encoding (plan §24) | Text Σ∞ form works; binary instruction encoding not implemented |
| `SigmaProgram` serialization | No `Serialize`/`Deserialize` on `SigmaProgram` |

### a2x-core (2 gaps)

| Area | Status |
|------|--------|
| `Packet` enum | No `Serialize`/`Deserialize` — can't serialize over wire via unified type |
| `OmegaProgram` trait | Not defined in core (plan says it should be) |

### a2x-bus (2 gaps)

| Area | Status |
|------|--------|
| TCP transport in async bus | Sync TCP transport exists; not integrated into `InMemoryAsyncBus` |
| Bounded channels | `InMemoryBus` uses `mpsc::UnboundedSender` — no backpressure |

### a2x-probe, a2x-cli, a2x-client (0 additional stubs)

These crates are generally well-implemented relative to plan.

### Stub Summary

| Crate | Stubs |
|-------|:-----:|
| a2x-ccs | 14 |
| a2x-agents | 7 |
| a2x-omega | 5 |
| a2x-gateway | 4 |
| a2x-sigma | 2 |
| a2x-core | 2 |
| a2x-bus | 2 |
| a2x-probe | 0 |
| a2x-cli | 0 |
| a2x-client | 0 |
| **Total** | **36** |

Note: Round 1 claimed 32 stubs. Round 2 finds 36 by including implicit stubs (missing features, plan deviations) and gaps beyond explicit `stub`/`todo!()` markers. The difference is primarily in the omega compiler (5 vs 1) and core/sigma types.

---

## Part D: Omega Compilation Pipeline — Full 7-Stage Audit

Per plans/02-omega-compiler.md §3, the pipeline has 7 stages:

| Stage | Plan | Actual | Assessment |
|:-----:|------|--------|:----------:|
| **1. Lexer** | Trie-based Unicode matcher → `Vec<Token>` | ✅ In `a2x-sigma/tokenizer.rs`. Fuzzed. Proptested. | Complete |
| **2. Parser** | Token stream → `SigmaProgram` with label table | ✅ In `a2x-sigma/parser.rs`. Proptested. | Complete |
| **3. Semantic Analyzer** | Validate jump targets, sub-programs, types, contradictions | ❌ Comment says "stub — validate basic structure" with NO validation code | **Missing entirely** |
| **4. IR Generator** | `SigmaProgram` → IR graph with dataflow edges | ⚠️ `build_ir()` maps IntentOp→Opcode (10 mappings). But: control flow always `Vec::new()`, only `operators[0]` used, no label resolution | **Major gaps** |
| **5. Optimizer** | 4 passes: constant folding, dead code, fusion, layout | ✅ All 4 passes exist with tests. Logic verified correct for basic cases. | Complete |
| **6. Code Generator** | IR graph → Ω tensors via topological sort | ⚠️ `codegen()` iterates nodes directly (no topological sort). `encode_instruction()` uses deterministic Blake3 hashing — works but lossy. Only Ω_I recoverable | **Gap: no dataflow ordering** |
| **7. Serializer** | Ω program → binary blob | ✅ `OmegaProgram` derives `Serialize`/`Deserialize`. Wire format tests exist. | Complete |

### Optimizer Pass Audit (Stage 5)

All 4 passes have tests and appear correct for basic cases:

| Pass | Tests | Logic Quality |
|------|:-----:|:-------------:|
| `constant_folding.rs` | 5 tests | ✅ Correct — folds all-immediate Bind into Nop |
| `dead_code.rs` | 7 tests | ✅ Correct — flat sequential programs retained, orphaned nodes with control flow removed |
| `fusion.rs` | 8 tests | ✅ Correct — merges adjacent Bind+Diff with matching labels, inherits control flow |
| `layout.rs` | 6 tests | ✅ Correct — sorts by source_index, stable, idempotent, nodes without source go last |

---

## Part E: Agent Implementation — Plan vs Reality

Per plans/05-agents.md, 5 agent types are defined. Here's the implementation status:

| Agent | Implementation | Key Gaps |
|-------|:-------------:|----------|
| **Orchestrator** | ✅ `dispatch()` runs programs on local VM. State summary works. | Can't dispatch to other agents over bus. Capability-based routing missing. |
| **CLI Agent** | ✅ VM execution, sandbox allowlist. Validates packets. | No real shell command execution. Container/VM sandbox stubs. No forbidden_commands denylist. |
| **LLM Agent** | ❌ Pure stub | `nl_to_sigma()` empty. `sigma_to_nl()` placeholder. `execute()` empty. No LLM API integration. |
| **CCS Agent** | ✅ Cognitive loop (EVOLVE→REFLECT→PLAN) via `tick()`. Graph grows per tick. | `query()` stub. No bus integration. `start_cognitive_loop()` marks bool, no thread. |
| **Ω Agent** | ❌ **Doesn't exist at all** | Plan defines "pure latent execution, max speed, zero inspectability". Not implemented. |

### Lifecycle State Machine

| State | Implemented | Plan Compliance |
|-------|:----------:|:---------------:|
| `Idle` | ✅ | ✅ |
| `Running { program_id, started_at }` | ✅ | ⚠️ Missing `vm: Box<CcsVm>` field per plan |
| `Error { error, retry_count }` | ✅ | ✅ |
| `Halted` | ✅ | ✅ |
| `Dead` | ✅ | ✅ |

Heartbeat tracking implemented. No bus heartbeat integration. No disk persistence.

---

## Part F: Security Model — Full Plan Audit

Per plans/12-security.md, there are 15 checklist items. Implementation status:

| # | Feature | Plan § | Status |
|:-:|---------|:------:|:------:|
| 1 | Agent Ed25519 key pair generation | §2 | ❌ Not implemented |
| 2 | Bus message signing + verification | §2 | ❌ Not implemented |
| 3 | Gateway API key authentication | §3 | ✅ `InMemoryAuthProvider` works |
| 4 | JWT token authentication | §3 | ❌ Stub: "accept any non-empty token" |
| 5 | Entity permission model (read/write/exec/admin) | §4 | ⚠️ Struct exists, never enforced |
| 6 | CLI agent command filtering | §5 | ✅ Basic allowlist; no denylist |
| 7 | Rate limiting (token bucket) | §5 | ❌ Not implemented |
| 8 | Resource limits on CLI agent | §5 | ❌ Not implemented |
| 9 | Audit logging (SecurityEvent enum) | §7 | ⚠️ `tracing` only; no structured events |
| 10 | Key rotation mechanism | §6 | ❌ Not implemented |
| 11 | TLS for bus transport | §2 | ❌ Not implemented |
| 12 | TLS for gateway HTTP/WS | §3 | ❌ Not implemented |
| 13 | Secure key storage (file permissions) | §6 | ❌ Not implemented |
| 14 | `forbidden_commands` denylist (rm -rf, sudo, etc.) | §5 | ❌ Not implemented |
| 15 | Graceful auth failure handling | §3 | ✅ Lock errors return proper responses |

**Score: 3/15 implemented, 2 partial, 10 not started.**

---

## Part G: Sub-Plan Compliance Matrix

All 15 sub-plans cross-referenced against actual implementation:

| # | Plan | Covered in Code? | Key Gaps |
|:-:|------|:----------------:|----------|
| 01 | Sigma Language | ✅ Full | Binary ISA encoding per §24 not started |
| 02 | Ω Compiler | ⚠️ Partial | 3/7 stages complete; semantic analyzer, IR gen, codegen have gaps |
| 03 | CCS VM | ✅ Full | Operators are basic implementations (averaging, splitting) |
| 04 | Bus Protocol | ✅ Full | TCP transport not in async bus; unbounded channels |
| 05 | Agents | ⚠️ Partial | LLM agent pure stub; Ω agent missing; no bus dispatch in orchestrator |
| 06 | Entity Gateway | ⚠️ Partial | Listener registration missing; permission enforcement missing |
| 07 | Probe & Debug | ✅ Full | Tracer has minor opcode logging bug (BUG-007) |
| 08 | Ecosystem | ✅ Full | CI/CD workflows exist |
| 09 | Core Types | ✅ Full | `Packet` enum missing Serialize |
| 10 | Concurrency | 🔄 In progress | Scheduler scaffolded; CancellationToken, bounded channels missing |
| 11 | Startup & Shutdown | ❌ **Not started** | No config loading, no boot order, no graceful shutdown, no PID file |
| 12 | Security | ❌ **3/15 items** | See Part F above |
| 13 | Documentation | ❌ **Minimal** | No mdbook, no examples/, stale README, no `.a2x-context.md` |
| 14 | Resilience | ⚠️ Partial | Watchdog step counter in VM; no crash recovery, no storage integrity, no resource exhaustion handling |
| 15 | WASM Support | ❌ **Not started** | No WASM target, no IndexedDB storage, no WebSocket transport |

### Plans with Zero Implementation

Three plans have essentially zero code:
- **Plan 11 (Startup & Shutdown)** — No config loading, no boot order, no graceful shutdown, no PID file, no `~/.a2x/` directory creation
- **Plan 13 (Documentation)** — No mdbook, no `examples/`, no `docs/`, no `scripts/`, stale README
- **Plan 15 (WASM)** — No WASM target, no `wasm-bindgen`, no `web-sys`, no WebSocket transport

---

## Part H: Serialization & Wire Format Audit

| Type | Crate | Serialize | Deserialize | Wire Format |
|------|-------|:---------:|:-----------:|-------------|
| `ConceptVector` | a2x-core | ✅ (serde) | ✅ | — |
| `RelationEdge` | a2x-core | ✅ (serde) | ✅ | — |
| `SigmaPacket` | a2x-sigma | ✅ (serde) | ✅ | Display trait for text |
| `SigmaProgram` | a2x-sigma | ❌ | ❌ | Can't serialize programs |
| `OmegaPacket` | a2x-omega | ✅ (serde) | ✅ | Bincode-ready |
| `OmegaProgram` | a2x-omega | ✅ (serde) | ✅ | Tested via omega_wire_roundtrip |
| `WireMessage` | a2x-bus | Custom | Custom | Length-prefixed binary frames |
| `Packet` (unified) | a2x-core | ❌ | ❌ | **Critical gap** — no unified wire format |

### Filesystem Serialization
- No `bincode` usage for WorldGraph/MemoryTrace persistence (plan §11)
- No `serde_json` usage for config files (plan §11)
- `blake3` used for ProgramId hashing ✅

---

## Part I: Competitive Protocol Research — Design Patterns to Adopt

Research on MCP (Anthropic), A2A (Google), ACP (Linux Foundation), ANP, and FIPA ACL:

| Pattern | Source | A2X Status | Recommendation |
|---------|--------|:----------:|----------------|
| **Agent Card** (typed metadata) | A2A | ❌ Agents have string capabilities only | Add `AgentCard` struct: capabilities, endpoints, auth, modalities, version |
| **Initialization handshake** | MCP + A2A | ❌ No capability negotiation | Add `AgentHandshake` phase before program execution |
| **Task lifecycle** (submitted→working→input-required→completed/failed) | A2A | ⚠️ Scheduler has Queued/Running but no `input-required` | Add `input-required` for human-in-the-loop |
| **SSE streaming** | MCP | ❌ No streaming support | Add SSE for long-running Σ∞ program progress updates |
| **Decentralized identity (DIDs)** | ANP | ❌ Agents have string IDs | Plan 12's Ed25519 keys align — implement them |
| **Speech act mapping** | FIPA ACL | ⚠️ IntentOp operators map implicitly | Formalize `IntentOp ↔ Speech Act` reference table |
| **JSON-RPC 2.0** | MCP + A2A | ❌ A2X has custom wire format | Consider JSON-RPC for MCP bridge compatibility |

### A2X Positioning

After competitive research, A2X's unique niche is clear:

> **A2X is the only system where AI agents write, compile, and execute programs in a language designed for machine cognition — with a differentiable runtime that learns from execution.**

It is not a protocol (like MCP/A2A/ACP), not a framework (like LangChain/CrewAI), and not a binary format (like WASM). It's a **language** — one that protocols can carry, frameworks can use, and binaries can optimize.

---

## Part J: Architecture Deviations from PLAN

| Deviation | PLAN Says | Code Does |
|-----------|-----------|-----------|
| Entity listener crates | Separate crates: `a2x-entity-http`, `a2x-entity-ws`, etc. | Modules in `a2x-gateway/src/listeners/` |
| `examples/` directory | PLAN Appendix C lists `examples/` with 5 example files | Directory does not exist |
| `docs/` directory | PLAN Appendix C lists `docs/` with 3 protocol reference docs | Directory does not exist |
| `scripts/` directory | PLAN Appendix C lists `scripts/setup-hooks.sh` | Directory does not exist |
| `a2x-omega/src/encoder.rs` | PLAN lists separate encoder file | File does not exist; encoder is inline in `compiler.rs`. Learned encoder in `learned_encoder.rs` |
| `a2x-omega/src/ir/mod.rs` | File picker found this path | File does not exist. IR types are inline or in separate files |
| `AgentState::Running{vm}` | Plan specifies `vm: Box<CcsVm>` | Implementation has no `vm` field |

---

## Part K: Dead Code Inventory

Files/items marked `#[allow(dead_code)]` or never called:

| Location | Item | Notes |
|----------|------|-------|
| `a2x-gateway/src/listeners/http.rs:263` | `HttpListener::router()` | Builds axum Router, never called |
| `a2x-gateway/src/listeners/http.rs:278` | `AuthQuery` struct | Defined, never used |
| `a2x-gateway/src/listeners/ws.rs:24` | `incoming_tx` field | Stored, never used |
| `a2x-gateway/src/listeners/ws.rs:26` | `response_rx` field | Stored, never used |
| `a2x-gateway/src/listeners/tcp.rs:18` | `incoming_tx` field | Stored, never used |
| `a2x-gateway/src/listeners/tcp.rs:20` | `response_rx` field | Stored, never used |
| `a2x-ccs/src/parallel_swarm.rs:32` | `max_depth` field | Stored, never used |

---

## Part L: Resilience & Watchdog Status

Per plans/14-resilience.md:

| Feature | Status |
|---------|:------:|
| Agent crash detection | ❌ Not implemented |
| Crash recovery sequence | ❌ Not implemented |
| Bus failure handling | ⚠️ `SendError::ReceiverDropped` handled but no reconnect logic |
| Gateway retry policy | ❌ Not implemented |
| Webhook redelivery | ❌ Not implemented |
| Watchdog timer | ⚠️ `CcsVm.watchdog_steps` exists (set via probe) but no wall-clock timeout |
| Instruction-level fault tolerance | ❌ No retry/skip/fallback |
| Storage corruption recovery | ❌ No atomic writes, no checksum verification, no backup fallback |
| Resource exhaustion handling | ❌ No memory pressure, no throttling, no eviction |
| Graceful degradation | ❌ Not implemented |
| `Suspend`/`Resume` VM status | ❌ Not implemented |

---

## Part M: Updated Priority Roadmap

### Immediate (This Week) — 6 items

| # | ID | What | Effort |
|---|----|------|--------|
| 1 | T6-4 | Update README project status | 30 min |
| 2 | T2-1 | Add `AgentCard` struct (A2A pattern) | 3 hours |
| 3 | BUG-005 | Wire permission enforcement into auth flow | 2 hours |
| 4 | T3-4 | Add `forbidden_commands` denylist to CLI agent | 1 hour |
| 5 | BUG-002 | Fix scheduler status transitions | 1 hour |
| 6 | T6-1 | Create `examples/` directory with 3 demos | 3 hours |

### Short-term (Next 2 Weeks) — 7 items

| # | ID | What | Effort |
|---|----|------|--------|
| 7 | T2-1 | Implement semantic analyzer (compiler Stage 3) | 5 hours |
| 8 | BUG-001 | Add `Gateway::add_listener()` method | 1 hour |
| 9 | T1-2 | Add `AgentHandshake` phase | 4 hours |
| 10 | T1-3 | Add `input-required` to scheduler + `Suspend`/`Resume` VM status | 1 day |
| 11 | T3-2 | Add bus-based dispatch to orchestrator | 4 hours |
| 12 | T4-3 | Add rate limiting (token bucket) | 2 hours |
| 13 | T5-1 | Add `CancellationToken` to scheduler | 2 hours |

### Medium-term (Next Month) — 6 items

| # | ID | What | Effort |
|---|----|------|--------|
| 14 | T3-1 | Implement LLM agent with real API integration | 2 days |
| 15 | T2-2 | Wire IR generator control flow | 4 hours |
| 16 | T4-1 | Implement Ed25519 agent identity + message signing | 1 day |
| 17 | T2-4 | Handle multi-operator intents in compiler | 2 hours |
| 18 | T6-2 | Build MCP bridge prototype | 1 day |
| 19 | T5-3 | Bounded channels on bus | 1 hour |

### Long-term (Next Quarter) — 5 items

| # | ID | What | Effort |
|---|----|------|--------|
| 20 | T3-3 | Implement Ω Agent type | 3 hours |
| 21 | — | Security hardening (TLS, key rotation, audit logging) | 1 week |
| 22 | — | Python/JavaScript client SDKs | 2 weeks |
| 23 | — | Web dashboard (WASM) | 2 weeks |
| 24 | — | End-to-end demo pipeline (HTTP → gateway → bus → CLI → result) | 3 days |

---

## Part N: Comprehensive Code Quality Scorecard (v2)

| Category | Score | Round 1 | Notes |
|----------|:-----:|:-------:|-------|
| **Architecture** | 8/10 | 8/10 | Clean layer separation. 3 plan deviations (listener crates, directories, encoder file) |
| **Type Safety** | 8/10 | 9/10 | Downgraded: `Packet` missing Serialize, `unwrap()` in production code, `try_into().unwrap()` on network input |
| **Test Coverage** | 7/10 | 7/10 | 499 tests across all crates. Good unit coverage. No integration demos. Fuzz tests exist for sigma. |
| **Documentation** | 4/10 | 5/10 | Downgraded: README stale, no examples, no mdbook, 4 sub-plans not referenced in code, no `.a2x-context.md` |
| **Error Handling** | 4/10 | 6/10 | Downgraded: 7 planned error variants never constructed. Semantic errors never produced. |
| **Stub Completeness** | 3/10 | 4/10 | Downgraded: LLM agent pure stub, Ω agent doesn't exist, semantic analyzer missing. 36 stubs found vs 32 claimed. |
| **Safety** | 3/10 | 3/10 | No change — `SafetyLevel::Bounded` is counter-only stub |
| **Concurrency** | 5/10 | 5/10 | Scheduler works but no CancellationToken, unbounded channels, status bugs |
| **Security** | 2/10 | — | New category. 3/15 plan items implemented |
| **Compiler Completeness** | 4/10 | — | New category. 3/7 stages complete, 3 partial, 1 missing |

**Overall: 4.8/10** (down from Round 1's 6.0 — the deeper audit across all 10 crates and 15 sub-plans reveals significantly more gaps)

---

## Part O: Quick Reference — All Fix IDs

| ID | Severity | Area | Description |
|----|:--------:|------|-------------|
| BUG-001 | 🔴 | Gateway | No `add_listener()` — gateway inert |
| BUG-002 | 🔴 | Scheduler | Status never → `Completed` |
| BUG-003 | 🟡 | TCP | `.unwrap()` on malformed frames |
| BUG-004 | 🟡 | Gateway | Mutex `.unwrap()` panics |
| BUG-005 | 🟡 | Security | Permissions never enforced |
| BUG-006 | 🟡 | Gateway | 7 dead code items |
| BUG-007 | 🟢 | Probe | Tracer logs wrong opcode |
| T1-1 | 🔴 | Protocol | Add `AgentCard` struct |
| T1-2 | 🟡 | Protocol | Add `AgentHandshake` |
| T1-3 | 🟡 | Protocol | Add `input-required` status |
| T2-1 | 🔴 | Compiler | Implement semantic analyzer |
| T2-2 | 🟡 | Compiler | Wire IR control flow |
| T2-3 | 🟢 | Compiler | Topological sort in codegen |
| T2-4 | 🟡 | Compiler | Handle multi-operator intents |
| T3-1 | 🔴 | Agents | Implement LLM agent with API |
| T3-2 | 🟡 | Agents | Bus dispatch in orchestrator |
| T3-3 | 🟢 | Agents | Implement Ω Agent |
| T3-4 | 🟡 | Security | Add `forbidden_commands` denylist |
| T3-5 | 🟢 | Agents | Add `Box<CcsVm>` to lifecycle |
| T4-1 | 🟡 | Security | Ed25519 agent identity |
| T4-2 | 🟡 | Security | Wire permission enforcement |
| T4-3 | 🟡 | Security | Rate limiting (token bucket) |
| T4-4 | 🟢 | Security | SecurityEvent audit logging |
| T5-1 | 🟡 | Async | CancellationToken to scheduler |
| T5-2 | 🟢 | Async | Bounded channels on bus |
| T5-3 | 🟡 | Async | Suspend/Resume VM status |
| T5-4 | 🟡 | Async | Fix status transitions |
| T6-1 | 🔴 | Docs | Create `examples/` directory |
| T6-2 | 🟡 | Ecosystem | MCP bridge prototype |
| T6-3 | 🟢 | Docs | `.a2x-context.md` for LLM discovery |
| T6-4 | 🟡 | Docs | Update README project status |

---

## Part P: File Tree Cross-Reference — PLAN vs Reality

Directories from PLAN Appendix C that don't exist:

```
❌ examples/          — PLAN lists 5 example files
❌ docs/              — PLAN lists 3 protocol reference docs
❌ scripts/           — PLAN lists setup-hooks.sh
❌ a2x-omega/src/encoder.rs      — PLAN lists separate file
❌ a2x-omega/src/ir/mod.rs       — File picker found this path
```

Files that exist but not in PLAN:

```
✅ a2x-ccs/src/async_vm.rs       — New in Phase 7 scaffolding
✅ a2x-ccs/src/parallel_swarm.rs — New in Phase 7 scaffolding
✅ a2x-ccs/src/scheduler.rs      — New in Phase 7 scaffolding
✅ a2x-bus/src/async_bus.rs      — New in Phase 7 scaffolding
✅ a2x-ccs/src/state_ndarray.rs  — ndarray backend (behind feature)
✅ a2x-omega/src/environment.rs  — Simulated training environment
✅ a2x-omega/src/training.rs     — Training loop
✅ a2x-omega/src/learned_decoder.rs — Learned decoder
✅ a2x-omega/src/learned_encoder.rs — Learned encoder
```

---

*Web research sources:*
- MCP spec: modelcontextprotocol.io/specification/2025-11-25
- A2A: developers.googleblog.com (A2A announcement), galileo.ai, mindstudio.ai, mindset.ai
- ACP: agentcommunicationprotocol.dev
- ANP: agent-network-protocol.com
- FIPA ACL: fipa.org, towardsai.net
- Tokio patterns: tokio.rs/blog/2020-04-preemption, docs.rs/tokio
- Linux Foundation A2A: linuxfoundation.org/press
- Zylos Research: zylos.ai/research agent protocol comparison

*This document supersedes earlier work reports for audit findings. Use this as the single source of truth for improvement planning.*
