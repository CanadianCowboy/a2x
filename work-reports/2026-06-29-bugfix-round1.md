# A2X Bugfix Round 1 — Comprehensive Codebase Audit

> **Date:** 2026-06-29
> **Scope:** Full workspace — all 10 crates, all plans, all work reports
> **Baseline:** v0.6.0 (commit `cf0ce6b`) + uncommitted Phase 7 async scaffolding
> **Purpose:** Document every known issue, debt, and gap before starting Phase 7 completion

---

## Executive Summary

| Category | Count | Severity |
|----------|-------|----------|
| **Compilation errors** | 2 | 🔴 Critical |
| **Failing tests** | 2 | 🔴 Critical |
| **Clippy warnings** | 0 | ✅ Clean |
| **Rustdoc warnings** | 5 | 🟡 Medium |
| **Design gaps (Phase 6 reviewer)** | 6 | 🟡 Medium |
| **Stub/incomplete implementations** | 12 | 🟢 Expected (Phase N+1) |
| **Unsafe unwrap/expect in prod code** | 3 | 🟡 Medium |
| **Missing features per plans** | ~20 | 🟢 Expected (future phases) |
| **Total LOC** | ~19,488 | across 10 crates |

---

## 🔴 CRITICAL — Compilation & Test Failures

### BUG-001: `VmStatus` undeclared in `parallel_swarm.rs` tests

- **File:** `crates/a2x-ccs/src/parallel_swarm.rs:267,294`
- **Error:** `error[E0433]: failed to resolve: use of undeclared type VmStatus`
- **Cause:** The `#[cfg(test)] mod tests` block uses `VmStatus::Halted` but doesn't import it. The `super::*` import doesn't bring `VmStatus` into scope because it's defined in `crate::vm`, not `crate::parallel_swarm`.
- **Fix:** Add `use crate::vm::VmStatus;` to the test module.
- **Status:** Already partially fixed (import added) but `cargo test --workspace` still fails — likely the auto-fix didn't apply correctly or the clippy auto-fix removed it.

### BUG-002: `test_scheduler_at_capacity` and `test_scheduler_cleanup` failing

- **File:** `crates/a2x-ccs/src/scheduler.rs:296,311`
- **Error:** Assertion failures in scheduler tests
- **Cause:** The `submit()` method spawns a tokio task that runs a VM, but the capacity check counts "Running" entries. The spawned task completes nearly instantly for NOP programs, so by the time the third `submit()` is called, the first two may have already completed and their entries still exist but status is still `Running` (never updated to `Completed`). The cleanup test expects `total_count()` to be 0 after `cleanup()`, but completed programs stay in the map because their status is never transitioned.
- **Fix:** Update task completion to set status to `Completed`, and have cleanup remove `Completed`/`Failed`/`Cancelled` entries.
- **Status:** Needs fix.

---

## 🟡 MEDIUM — Rustdoc Warnings

### DOC-001: Unresolved links in `a2x-ccs`

- **File:** `crates/a2x-ccs/src/probe.rs`
- **Warning:** 4 unresolved link warnings referencing `i` and `0`
- **Cause:** Markdown link syntax `[i]` and `[0]` in doc comments being interpreted as intra-doc links.
- **Fix:** Escape with backticks: `` [`i`] `` and `` [`0`] ``

### DOC-002: Unclosed HTML tag in `a2x-gateway`

- **File:** `crates/a2x-gateway/src/lib.rs` (or similar)
- **Warning:** `Unclosed HTML tag Mutex`
- **Cause:** `<Mutex<GatewayState>>` in doc comment parsed as HTML.
- **Fix:** Use backtick fencing: `` `Mutex<GatewayState>` ``

---

## 🟡 MEDIUM — Design Gaps (Phase 6 Code Reviewer Findings)

### DESIGN-001: `std::sync::Mutex` in async HTTP handlers

- **File:** `crates/a2x-gateway/src/listeners/http.rs`
- **Issue:** `handle_execute`, `handle_list_entities`, etc. lock `state.gateway` (a `std::sync::Mutex`) inside async axum handlers. This blocks the tokio thread during lock contention.
- **Fix:** Switch to `tokio::sync::Mutex` for `GatewayState` or restructure to avoid holding locks across await points.
- **Priority:** High — will cause real performance issues under load.

### DESIGN-002: No way to register listeners on the gateway

- **File:** `crates/a2x-gateway/src/gateway.rs`
- **Issue:** `GatewayState.listeners` is always empty — there's no public `add_listener()` method, so the gateway is structurally complete but functionally inert. `start()`/`stop()` iterate over an empty vec.
- **Fix:** Add `Gateway::add_listener(Box<dyn ProtocolListener>)`.

### DESIGN-003: `EntityPermissions` never enforced in auth flow

- **File:** `crates/a2x-gateway/src/auth.rs`
- **Issue:** `InMemoryAuthProvider` stores permissions but `authenticate()` doesn't check them. Rate limits, instruction caps, and probe access are dead configuration.
- **Fix:** Add permission checking in the gateway's request handlers.

### DESIGN-004: Dead code fields in WS/TCP listeners

- **Files:** `crates/a2x-gateway/src/listeners/ws.rs`, `tcp.rs`
- **Issue:** `incoming_tx`/`response_rx` fields stored with `#[allow(dead_code)]` but never used in any method.
- **Fix:** Either wire them into the listener logic or remove them.

### DESIGN-005: `router()` method builds axum Router but is never called

- **File:** `crates/a2x-gateway/src/listeners/http.rs`
- **Issue:** `HttpListener::router()` builds an axum Router but is never called. Dead code.
- **Fix:** Expose it for integration testing or remove.

### DESIGN-006: `AuthQuery` struct defined but never used

- **File:** `crates/a2x-gateway/src/listeners/http.rs`
- **Issue:** `AuthQuery` struct is defined but never used in any handler.
- **Fix:** Remove or wire into auth middleware.

---

## 🟡 MEDIUM — Unsafe `unwrap()`/`expect()` in Production Code

### UNWRAP-001: `world_graph.rs` lookup unwrap

- **File:** `crates/a2x-ccs/src/world_graph.rs`
- **Issue:** `self.graph.node_weight(ni).unwrap()` in `lookup()` — after confirming `node_index(id)` returns `Some(ni)`, the weight is unwrapped. This is safe in practice (StableGraph guarantees weight exists for valid NodeIndex), but the `expect()` message should be more descriptive.
- **Priority:** Low — correct but fragile.

### UNWRAP-002: `tcp_transport.rs` byte array conversions

- **File:** `crates/a2x-bus/src/tcp_transport.rs`
- **Issue:** `try_into().unwrap()` for `[u8; 4]` and `[u8; 8]` conversions in `decode_message()`. If the frame is malformed, this could panic.
- **Fix:** Use `.map_err()` instead of `.unwrap()`.
- **Priority:** Medium — can panic on malformed network input.

### UNWRAP-003: `gateway.rs` mutex locks

- **File:** `crates/a2x-gateway/src/gateway.rs`
- **Issue:** `self.state.lock().unwrap()` in multiple methods. If the mutex is poisoned (thread panicked while holding the lock), this will panic.
- **Fix:** Use `.lock().map_err(|e| GatewayError::...)` consistently.
- **Priority:** Low — mutex poisoning is rare.

---

## 🟢 EXPECTED — Stub/Incomplete Implementations

These are known stubs documented in work reports and plans. They are NOT bugs — they are planned future work.

### STUB-001: JWT authentication stub
- **File:** `crates/a2x-gateway/src/auth.rs`
- **Status:** Accepts any non-empty token. Real JWT validation deferred.

### STUB-002: LLM Agent `nl_to_sigma` / `sigma_to_nl`
- **File:** `crates/a2x-agents/src/llm_agent.rs`
- **Status:** Returns empty packets. Phase 4+ for learned translation.

### STUB-003: CLI Agent `execute` — no real shell execution
- **File:** `crates/a2x-agents/src/cli_agent.rs`
- **Status:** Returns empty raw packet. Phase 1 added tracing but no real command execution.

### STUB-004: Orchestrator `dispatch` — no real multi-agent routing
- **File:** `crates/a2x-agents/src/orchestrator.rs`
- **Status:** Runs program on local VM only. Phase 6+ for bus-based dispatch.

### STUB-005: PolicyField `evaluate` — stub uniform distribution
- **File:** `crates/a2x-ccs/src/policy.rs`
- **Status:** Returns uniform [0.25, 0.25, 0.25, 0.25]. Phase 4+ for learned policy.

### STUB-006: `from_snapshot` — minimal VM snapshot restore
- **File:** `crates/a2x-ccs/src/parallel_swarm.rs`
- **Status:** Creates fresh VM, ignores graph data. Full serialization deferred to Phase 8.

### STUB-007: `merge_swarm_results` — no actual merge
- **File:** `crates/a2x-ccs/src/parallel_swarm.rs`
- **Status:** Logs debug message, no state mutation. Full merge strategy deferred.

### STUB-008: `serialize_world_graph` — 8-byte stub
- **File:** `crates/a2x-ccs/src/parallel_swarm.rs`
- **Status:** Returns only node_count + edge_count. Full bincode serialization deferred.

### STUB-009: `inspect` on `GraphQuery::Custom(_)`
- **File:** `crates/a2x-ccs/src/world_graph.rs`
- **Status:** Returns empty vec. Phase 8+ for custom query DSL.

### STUB-010: `send_probe_event` always returns `Ok(())`
- **File:** `crates/a2x-ccs/src/vm.rs`
- **Status:** Uses `let _ = tx.send(event)` — silently drops send errors. Acceptable for fire-and-forget events.

### STUB-011: `Opcode::Nop` in `run()` tracer logs wrong opcode
- **File:** `crates/a2x-ccs/src/vm.rs` (in `run()` method)
- **Status:** After `step()`, the tracer entry logs `Opcode::Nop` because the opcode was consumed. `run_probed()` captures it correctly before step.

### STUB-012: `ProbeExt` trait defined but never implemented
- **File:** `crates/a2x-probe/src/lib.rs`
- **Status:** No concrete implementations. Phase 6+ for bus-based probe.

---

## 🟢 EXPECTED — Features Missing Per Plans (Future Phases)

### PLAN-GAP-001: Phase 7 Concurrency (in progress)
- **Status:** Async bus, async VM, scheduler, parallel swarm created but have compilation/test issues.

### PLAN-GAP-002: Phase 8 — Startup/Shutdown (plans/11)
- **Missing:** Config loading, ordered boot sequence, graceful shutdown, state persistence, PID files.

### PLAN-GAP-003: Phase 9 — Security (plans/12)
- **Missing:** Ed25519 agent identity, bus message signing, TLS, JWT validation, rate limiting, audit logging.

### PLAN-GAP-004: Phase 10 — Documentation (plans/13)
- **Missing:** mdbook, rustdoc CI, examples, protocol reference.

### PLAN-GAP-005: Phase 11 — Resilience (plans/14)
- **Missing:** Crash recovery, watchdog timers, retry policies, storage corruption handling.

### PLAN-GAP-006: Phase 12 — WASM (plans/15)
- **Missing:** Browser-based agents, web dashboard, WASM build pipeline.

### PLAN-GAP-007: Cross-machine demo (Phase 3.4)
- **File:** `crates/a2x-agents/tests/phase3_cross_machine.rs`
- **Status:** Test file exists but was never populated (empty or stub).

### PLAN-GAP-008: `a2x-entity-{http,ws,tcp,stdio}` crates
- **Status:** Plan mentions 4 entity protocol listener crates. They were inlined into `a2x-gateway/src/listeners/` instead.

### PLAN-GAP-009: Python/JavaScript client SDKs
- **Status:** Plan §30 mentions these as Phase 6 deliverables. Not started.

### PLAN-GAP-010: End-to-end demo
- **Status:** Plan §30 mentions "web app → HTTP → gateway → bus → CLI agent → result". Not implemented.

---

## 📊 Crate Health Summary

| Crate | LOC | Tests | Status | Notes |
|-------|-----|-------|--------|-------|
| a2x-core | 1,440 | 25 | ✅ Clean | Foundation types solid |
| a2x-sigma | 1,676 | 30+ | ✅ Clean | Tokenizer/parser complete, proptested |
| a2x-omega | 2,927 | 83+ | ✅ Clean | Compiler, decoder, learned modules |
| a2x-bus | 1,694 | 27 | ⚠️ 1 new | Async bus added, all tests pass |
| a2x-ccs | 7,092 | 158+ | 🔴 2 failing | Scheduler tests broken, VmStatus import |
| a2x-agents | 1,241 | 25 | ✅ Clean | All agents functional (stubs where planned) |
| a2x-cli | 630 | 22 | ✅ Clean | 4 subcommands working |
| a2x-gateway | 2,352 | 52 | ✅ Clean | Architecture solid, design gaps noted |
| a2x-client | 324 | 6 | ✅ Clean | SDK functional |
| a2x-probe | 1,112 | 36+ | ✅ Clean | Inspector, tracer, visualization |
| **Total** | **~19,488** | **~464+** | | |

---

## Recommended Fix Priority

### Immediate (must fix before Phase 7 commit)

1. **BUG-001:** Fix `VmStatus` import in `parallel_swarm.rs` tests
2. **BUG-002:** Fix scheduler test status transitions and cleanup logic

### Short-term (before v0.7.0 tag)

3. **DESIGN-001:** Switch HTTP handlers to `tokio::sync::Mutex`
4. **DESIGN-002:** Add `Gateway::add_listener()` method
5. **DOC-001/002:** Fix rustdoc warnings
6. **UNWRAP-002:** Fix TCP decode panics on malformed input

### Medium-term (next phase)

7. **DESIGN-003:** Wire permission enforcement into auth flow
8. **DESIGN-004/005/006:** Clean up dead code in gateway listeners
9. **UNWRAP-003:** Replace gateway mutex unwraps with error handling

### Long-term (future phases)

10. All PLAN-GAP items — these are planned future work, not bugs.

---

*This document is part of the A2X project. See PLAN.md for the full architecture.*
