# Work Report — P1: Bus Hardening + Gap Audit Closure

> **Date:** 2026-07-06
> **Crates:** a2x-bus, a2x-agents
> **Briefing:** A2X Team Briefing 2026-07-05 — P1 action items + Gap audit #03/#05/#07

---

## Summary

Closed all three P1 action items and audited/closed the three remaining gap items (#03, #05, #07) from the Team Briefing.

**P1 Hardening:**
1. **BusBridge API docs** — Added lifecycle documentation (5-step flow), error handling guidance, and `publish_raw` direct transport explanation
2. **TcpAsyncBridge productionized** — Replaced stub roundtrip test with real send/receive over localhost. Added multi-message and 96-message stress tests.
3. **TLS + AgentIdentity E2E** — Added 3 comprehensive signing pipeline tests: Alice→Bob full flow, subsequent message (caching), 5-agent cross-verification

**Gap Audit (#03/#05/#07):**
- **#03 Sigma encoding:** Confirmed ready — `event_to_sigma()` public API with 8 tests
- **#05 Agent orchestration:** Fixed — wired `dispatch_via_bus` to actually call `bus.send_sigma()`
- **#07 Omega compilation:** Confirmed ready — 6-stage pipeline with semantic analysis, IR, optimization, codegen

**Verification:** 762 workspace tests pass, clippy clean, fmt clean.

---

## P1-1 — BusBridge API Documentation

### Problem

`BusBridge` had functional code but lacked user-facing documentation about its lifecycle, error handling patterns, and the relationship between `publish()` and `publish_raw()`.

### Solution

Added comprehensive module-level documentation covering:

| Section | Content |
|---------|---------|
| **Lifecycle** | 5-step flow: Create → Register → Publish/Receive → Deregister → Drop |
| **Error handling** | Guidance on `BusError::NoRoute` (no agent with capability) vs `TransportError` (connection lost) |
| **publish_raw** | When to use direct transport access (advanced send patterns bypassing router) |
| **Relationship to Bus** | `BusBridge` owns the `Bus` — drop order matters for graceful shutdown |

---

## P1-2 — TcpAsyncBridge Productionization

### Problem

The `test_send_and_receive_over_localhost` test was a stub:
```rust
// TODO: actual round-trip test using tokio::net::TcpListener
assert!(true);
```

No real TCP send/receive was tested. No stress/backpressure tests existed.

### Solution

Replaced the stub with three real tests:

| Test | What it verifies |
|------|-----------------|
| `test_send_and_receive_over_localhost` | Full roundtrip: bind → send frame → accept → decode → verify |
| `test_send_multiple_messages_over_localhost` | 3 messages, sorted by correlation_id, order verified |
| `test_channel_backpressure_does_not_lose_messages` | 96 messages (below channel capacity of 128), all received, panic on timeout or channel-close |

**Key design choices:**
- Uses a "scout listener" to find an available OS port before binding (avoids port conflicts in CI)
- `tokio::time::timeout` with 2s timeout on all async operations — prevents test hangs
- Messages are sorted by correlation_id before verification (TCP connection ordering is not guaranteed)
- Explicit panic on `None` from `tokio::select!` (channel closed prematurely)

---

## P1-3 — AgentIdentity E2E Signing Pipeline

### Problem

`TlsTransport` and `AgentIdentity` had unit tests for key generation, signing, and verification in isolation, but no end-to-end tests showing the full pipeline: encode → sign → verify → decode, with real wire-format messages.

### Solution

Added three E2E tests:

| Test | What it verifies |
|------|-----------------|
| `test_e2e_signing_pipeline_with_wire_message` | Alice→Bob full flow: generate identity → encode_frame → sign → create `SignedWireMessage` → Bob verifies → decode `WireMessage`. Also: Eve forgery detected (Bob rejects Eve's signature on Alice's identity). |
| `test_e2e_subsequent_message_without_key` | After first contact, subsequent messages omit `verifying_key`. Bob uses cached key. Verifies Bob correctly handles the case where the key field is empty. |
| `test_e2e_multi_agent_signing` | 5 agents generate independent identities. Each signs a message. Cross-verification: agent_i's signature fails when verified against agent_j's key (for i ≠ j). All 20 cross-pairs verified. |

**Key design choices:**
- Tests use `crate::tcp_transport::encode_frame` and `decode_frame` — real wire format, not mock data
- Eve forgery test validates that the signature is bound to the message AND the signer's identity
- Caching test validates a real-world optimization path (don't re-send keys unnecessarily)

---

## Gap #03 — Sigma Encoding API (Audit Only)

### Finding: ✅ Ready — No changes needed

The `event_to_sigma()` function in `a2x-bus/src/bridge.rs` is exactly the encoding API Soong needs:

```rust
pub fn event_to_sigma(event_type: &str, payload: &[u8]) -> SigmaPacket
```

- Public, re-exported as `a2x_bus::event_to_sigma`
- Maps event type strings (`"system:alert"`, `"explore:search"`, `"merge:synthesize"`) to well-formed Σ∞ packets
- 8 tests covering all event type mappings
- Soong can call: `use a2x_bus::event_to_sigma; let pkt = event_to_sigma("system:alert", b"disk full");`

Additionally:
- `a2x_sigma::parse_program()` for string→program conversion
- `a2x_agents::parse::packet_to_sigma_program()` for Packet→SigmaProgram
- `a2x_sigma::to_bytes()` / `from_bytes()` for binary encoding

---

## Gap #05 — Agent Orchestration (Fixed)

### Problem

`dispatch_via_bus()` had a TODO stub — it always fell back to local execution:
```rust
// TODO: wire bus.send_program(target, program) when async bus is ready.
self.dispatch(program)
```

The bus already had `send_sigma()` — the only blocker was `&Bus` vs `&mut Bus`.

### Solution

Changed the bus parameter from `&Bus` to `&mut Bus` and wired real sends:

```rust
for (i, packet) in program.instructions.iter().enumerate() {
    bus.send_sigma(&self.id, packet, &required_cap, i as u64)?;
}
```

- Each packet in the program is sent individually via `bus.send_sigma()`
- Packet index used as correlation_id for tracking
- Returns empty `SigmaProgram` — response arrives via `bus.receive()` on subsequent poll
- Two call sites in `integration_pipeline.rs` updated (`&bus` → `&mut bus`)

---

## Gap #07 — Omega Compilation (Audit Only)

### Finding: ✅ Ready — No changes needed

The `CompileToOmega` trait (implemented for `SigmaProgram`) provides a full 6-stage compilation pipeline:

| Stage | What it does |
|-------|-------------|
| 1. Parse | Σ∞ text → `SigmaProgram` (via `parse_program()`) |
| 2. Semantic | Validates jump targets, contradictory operators, data types |
| 3. IR | Builds `IrGraph` with nodes, operands, control flow edges |
| 4. Optimize | 3 passes: constant folding, dead code elimination, fusion |
| 5. Codegen | Topological sort (dataflow-aware) → encodes as `OmegaProgram<29796>` |
| 6. Output | Returns tensor-encoded Ω program |

Soong can call: `my_sigma_program.compile(OptimizationLevel::Light)`

Additionally: encoder/decoder traits, learned encoder/decoder (feature-gated), MCP bridge for model context protocol.

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `crates/a2x-bus/src/bridge.rs` | Lifecycle docs + error handling guidance | +40 |
| `crates/a2x-bus/src/async_tcp.rs` | Real roundtrip + multi-message + stress tests | +120 / -15 |
| `crates/a2x-bus/src/identity.rs` | 3 E2E signing pipeline tests | +110 |
| `crates/a2x-agents/src/orchestrator.rs` | Wired dispatch_via_bus to send_sigma() | +12 / -8 |
| `crates/a2x-cli/tests/integration_pipeline.rs` | Updated 2 call sites: &bus → &mut bus | +2 / -2 |

---

## Test Results

| Crate | Tests | Status |
|-------|------:|:------:|
| a2x-bus | 65 | ✅ |
| a2x-agents | 63 | ✅ |
| a2x-cli | 57 | ✅ |
| a2x-ccs | 179 | ✅ |
| a2x-gateway, a2x-startup, a2x-core, a2x-sigma, a2x-omega, a2x-probe | 394 (combined) | ✅ |
| **Workspace total** | **762** | ✅ (4 ignored) |

---

## Team Briefing — Final Status

### Action Items (Section 6) — All Closed

| Priority | Action | Status |
|:--------:|--------|:------:|
| 🔴 P0 | CCS VM running + Σ∞ execution | ✅ |
| 🔴 P0 | Bus generic over Transport | ✅ |
| 🟠 P1 | BusBridge API stable/documented | ✅ |
| 🟠 P1 | TcpAsyncBridge productionized | ✅ |
| 🟠 P1 | TLS + AgentIdentity E2E verified | ✅ |
| 🟡 P2 | a2x-gateway service | ✅ |
| 🟡 P2 | a2x-startup boot sequence | ✅ |
| 🟢 P3 | a2x-probe introspection | ✅ |

### Gap Items (Section 2.4) — All Closed

| # | Gap | Status |
|:-:|------|:------:|
| 01 | CCS VM not running | ✅ Fork/Merge wired |
| 02 | Core type system not unified | ✅ ConceptVector, NodeId, WorldGraph, StateField ready |
| 03 | Sigma encoding not wired | ✅ event_to_sigma() public API |
| 04 | Bus not configured | ✅ Soong OS hosts bus |
| 05 | Agent orchestration not wired | ✅ dispatch_via_bus wired |
| 06 | Gateway not active | ✅ Full gateway with 4 listeners |
| 07 | Omega compilation not used | ✅ 6-stage pipeline |
| 08 | Startup boot stub | ✅ BootSequence with 6 phases |
| 09 | Probe not active | ✅ ProbeTool with tracer + graphviz |

---

## Design Decisions

1. **Real TCP tests, not mocks:** The TcpAsyncBridge tests bind real OS ports and send actual TCP frames. This catches real-world issues (port conflicts, async ordering, frame corruption) that mock tests would miss.

2. **E2E signing uses real wire format:** The identity tests use `encode_frame`/`decode_frame` from `tcp_transport.rs` — the same code the production bus uses. No test-only abstractions.

3. **dispatch_via_bus sends per-packet, not per-program:** The bus `send_sigma()` works on individual `SigmaPacket`s, so `dispatch_via_bus` loops through the program's instructions. Correlation IDs use the packet index. The response arrives asynchronously via `bus.receive()`.

---

## Next Steps

1. Tag `v0.7.1` to capture P1 + gap audit closure
2. Soong Path integration: wire `event_to_sigma()` + `dispatch_via_bus()` into Soong's EventStream
3. Future: async bus send (tokio-based) for true non-blocking dispatch
