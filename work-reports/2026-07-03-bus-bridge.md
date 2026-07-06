# Work Report — BusBridge Convenience Helper

> **Date:** 2026-07-03
> **Crate:** a2x-bus
> **Gap Report:** 2026-07-02-a2x-bus.md

---

## What Was Done

### New: `BusBridge` in `crates/a2x-bus/src/bridge.rs`

Implemented a convenience wrapper around `Bus` that addresses the gap report's need for
publishing domain events as Σ∞ packets and polling for incoming messages.

**Public API:**

| Method | Purpose |
|--------|---------|
| `BusBridge::new(bus, agent_id)` | Create a bridge wrapping a Bus |
| `register(agent_type, capabilities)` | Register this agent on the bus |
| `deregister()` | Remove this agent from the bus |
| `publish_sigma(packet, capability)` | Publish a pre-built Σ∞ packet |
| `publish_event(event_type, payload)` | Build Σ∞ from domain event + publish |
| `publish_raw(payload, msg_type, capability, recipient)` | Send raw WireMessage |
| `poll()` | Receive pending messages for this agent |
| `agent_id()`, `agent_count()`, `has_agent()` | Read-only accessors |
| `bus()`, `bus_mut()` | Access underlying Bus |

**Public free function:**

| Function | Purpose |
|----------|---------|
| `event_to_sigma(event_type, payload)` | Build a Σ∞ packet from a domain event string |

**`event_to_sigma` keyword → intent mapping:**

| Keywords | IntentOp |
|----------|----------|
| alert, error, critical, warning | Warning (⚠) |
| discover, explore, search | Star (✦) |
| merge, combine, synthesize, fuse | Synthesis (✣) |
| cancel, stop, halt | Cancel (✕) |
| parallel, fork | Parallel (⩫) |
| split, divide | Split (⩨) |
| delay, pause, wait | Delay (⧖) |
| accelerate, speed, fast | Accelerate (⧗) |
| (default) | Lightning (⚡) |

**Context mapping:** Event type segments (colon-separated) become context labels.
`cognition:*` → CausalChain, `system:*` → Universal, `detect:*`/`anomaly:*` → Uncertainty.

### Modified: `Bus` in `crates/a2x-bus/src/bus.rs`

Added `transport_mut()` accessor so `BusBridge` can send raw `WireMessage`s directly
to specific agents via the transport layer.

### Updated: `crates/a2x-bus/src/lib.rs`

Added `pub mod bridge;` and re-exports of `BusBridge` and `event_to_sigma`.

---

## Files Changed

| File | Change |
|------|--------|
| `crates/a2x-bus/src/bridge.rs` | **New** — 450 lines, BusBridge + event_to_sigma + 22 tests |
| `crates/a2x-bus/src/bus.rs` | +4 lines — `transport_mut()` accessor |
| `crates/a2x-bus/src/lib.rs` | +2 lines — bridge module + re-exports |
| `.a2x-context.md` | Updated bus file structure and test count (27→60) |

---

## Test Results

- `cargo test -p a2x-bus`: **60 passed** (59 unit + 1 doc-test)
- `cargo test --workspace`: **821 passed**, 4 ignored, 0 failed
- `cargo clippy -p a2x-bus -- -D warnings`: **clean**
- `cargo fmt -p a2x-bus`: **clean**

---

## Design Decisions

1. **`BusBridge` combines both publishing and receiving** rather than separate `BusBridge` + `BusListener` structs. Both share the same Bus instance, and the consumer typically needs both in one place. `poll()` covers the listener functionality.

2. **`publish_event` hardcodes `Capability::Execute`** — most domain events are execution requests. Consumers with custom routing can use `publish_sigma(packet, custom_capability)` directly.

3. **`publish_raw` with `None` recipient wraps the payload** as Σ∞ packet data rather than silently dropping it. The `msg_type` is encoded as a context label for dispatch.

4. **Keyword matching uses `contains()`** — simple and matches common event naming conventions (e.g., `"synthesize:thought"` matches Synthesis). Documented the trade-off with partial substring matches.
