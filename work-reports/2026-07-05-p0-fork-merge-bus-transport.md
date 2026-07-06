# Work Report — P0: Fork/Merge + Bus Transport Refactoring

> **Date:** 2026-07-05
> **Crates:** a2x-ccs, a2x-bus
> **Briefing:** A2X Team Briefing 2026-07-05 — P0 action items

---

## Summary

Closed both P0 gaps from the Team Briefing:

1. **CCS VM Fork/Merge** — `Opcode::Fork` and `Opcode::Merge` are no longer empty stubs. The VM now executes child programs sequentially via `VmSnapshot` forking, and merges results back. All plumbing moved from feature-gated `parallel_swarm.rs` into core `vm.rs`.

2. **Bus Transport Refactoring** — `Bus` and `BusBridge` are now generic over `T: Transport` with a default of `InMemoryTransport`. All existing callers work without changes. Added `with_transport()` constructors for custom backends (TCP, TLS, etc.).

**Verification:** 911 workspace tests pass, clippy clean, fmt clean.

---

## P0a — CCS VM Fork/Merge

### Problem

`Opcode::Fork` and `Opcode::Merge` were empty stubs in `CcsVm::step()`:
```rust
Opcode::Fork => {}  // dead
Opcode::Merge => {} // dead
```

Soong Path's cognitive pipeline requires parallel/swarm execution of sub-programs. The parallel swarm code (`execute_fork`, `merge_swarm_results`) existed in `parallel_swarm.rs` but was feature-gated behind `#[cfg(feature = "tokio")]` and never called from `step()`.

### Solution

**Design choice:** Sequential child execution in `step()` (not async tokio spawns). This keeps the core VM synchronous, deterministic, and compatible with `#![no_std]`/WASM. The async `execute_fork()` in `parallel_swarm.rs` remains for external callers that want true parallelism.

**Implementation:**

| Change | File | Detail |
|--------|------|--------|
| `VmSnapshot` struct | `vm.rs` | New — captures WorldGraph clone + StateField snapshot for child VM creation |
| `snapshot()` method | `vm.rs` | Serializes current VM state into `VmSnapshot` |
| `from_snapshot()` | `vm.rs` | Creates a child VM from a parent snapshot |
| `merge_swarm_results()` | `vm.rs` | Merges child graphs/states back into parent |
| `pending_merges` field | `vm.rs` | `Vec<VmSnapshot>` — stores child results between Fork and Merge |
| `Opcode::Fork` impl | `vm.rs` | Resolves sub-programs from context labels, snapshots parent, runs each child sequentially, stores results |
| `Opcode::Merge` impl | `vm.rs` | Calls `merge_swarm_results()` on all pending merges |
| Removed duplicates | `parallel_swarm.rs` | Moved snapshot/merge logic out (now thin async wrapper) |
| Safety allowlist | `safety.rs` | Added Fork and Merge opcodes to allowed set |

**Key design decisions:**
- Sequential execution (not tokio) — deterministic, no runtime dependency, simpler debugging
- `VmSnapshot` uses owned clones (not Arc) — explicit ownership, no shared state between parent and children
- `merge_swarm_results` merges graph nodes and state fields — preserves existing semantics from `parallel_swarm.rs`

### Verification

- `cargo test -p a2x-ccs`: **179 tests pass** (173 existing + 6 new Fork/Merge tests)
- Added tests: `test_fork_executes_subprograms`, `test_fork_no_subprograms`, `test_merge_collects_results`, `test_fork_merge_roundtrip`
- Safety allowlist updated to include Fork and Merge opcodes

---

## P0b — Bus Transport Refactoring

### Problem

`Bus` was hardcoded to `InMemoryTransport`:
```rust
pub struct Bus {
    transport: InMemoryTransport,  // concrete type
    ...
}
```

The Team Briefing requires `Bus` to support any `Transport` trait implementation (in-memory or TCP) — a prerequisite for bus graduation from Soong OS to dedicated infrastructure.

### Solution

Made `Bus` generic over `T: Transport` with a default of `InMemoryTransport`:

```rust
pub struct Bus<T: Transport = InMemoryTransport> {
    transport: T,
    discovery: InMemoryDiscovery,
    router: Router,
}
```

**API surface:**

| Constructor | Impl block | Purpose |
|-------------|-----------|---------|
| `Bus::new()` | `impl Bus<InMemoryTransport>` | Default in-memory bus (backward compat) |
| `Bus::with_strategy(s)` | `impl Bus<InMemoryTransport>` | In-memory with custom routing |
| `Bus::with_transport(t)` | `impl<T: Transport> Bus<T>` | Custom transport backend |
| `Bus::with_transport_and_strategy(t, s)` | `impl<T: Transport> Bus<T>` | Custom transport + routing |

`BusBridge` follows the same pattern:
```rust
pub struct BusBridge<T: Transport = InMemoryTransport> {
    bus: Bus<T>,
    ...
}
```

**Consumer impact: Zero.** All existing `Bus::new()` and `BusBridge::new(bus, id)` callers work without changes thanks to the default type parameter.

**`transport_mut()` return type:** Changed from `&mut InMemoryTransport` → `&mut T`. The only caller (`bridge.rs`) only calls `.send()` (a `Transport` trait method), so no breakage.

### Verification

- `cargo test --workspace`: **911 tests pass**, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings`: **clean**
- `cargo fmt`: **clean**
- Added tests: `test_with_transport_equivalent_to_new`, `test_with_transport_and_strategy`

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `crates/a2x-ccs/src/vm.rs` | Fork/Merge wiring, VmSnapshot, snapshot/merge methods, tests | +255 / -20 |
| `crates/a2x-ccs/src/parallel_swarm.rs` | Thinned to async wrapper only | +7 / -84 |
| `crates/a2x-ccs/src/safety.rs` | Added Fork/Merge to allowlist | +7 |
| `crates/a2x-ccs/src/lib.rs` | Export `VmSnapshot` | +1 / -1 |
| `crates/a2x-bus/src/bus.rs` | Generic `Bus<T: Transport>`, with_transport constructors, tests | +80 / -7 |
| `crates/a2x-bus/src/bridge.rs` | Generic `BusBridge<T: Transport>`, updated accessor types | +16 |

---

## Test Results

| Crate | Tests | Status |
|-------|------:|:------:|
| a2x-ccs | 179 | ✅ |
| a2x-bus | 61 | ✅ |
| a2x-gateway | 65 | ✅ |
| a2x-agents | 62 | ✅ |
| a2x-cli | 22 | ✅ |
| a2x-startup | 70 | ✅ |
| a2x-sigma, a2x-omega, a2x-core, a2x-probe | 452 (combined) | ✅ |
| **Workspace total** | **911** | ✅ |

---

## Design Decisions

1. **Sequential Fork (not async):** The core `CcsVm::step()` runs child programs sequentially rather than spawning tokio tasks. This keeps the VM deterministic, `#![no_std]` compatible, and debuggable. External callers can still use `execute_fork()` for async parallelism via the `tokio` feature gate.

2. **VmSnapshot lives in vm.rs, not parallel_swarm.rs:** Moved snapshot/merge primitives out of the feature-gated module so Fork/Merge works without tokio. `parallel_swarm.rs` is now a thin async wrapper.

3. **Default type parameter for Bus:** `Bus<T: Transport = InMemoryTransport>` preserves all existing code. No consumer needs to add type annotations. `with_transport()` is additive — only used when a non-default transport is needed.

4. **Separate impl blocks for default vs generic constructors:** `new()` and `with_strategy()` only exist for `Bus<InMemoryTransport>`. Calling `Bus::<SomeOtherTransport>::new()` produces a clear compiler error directing users to `with_transport()` instead.

---

## Next Steps

1. Commit Fork/Merge + Bus transport changes with conventional commit messages
2. P1: Stabilize `BusBridge` API documentation
3. P1: Productionize `TcpAsyncBridge` for external agent connections
4. P1: Verify `TlsTransport` + `AgentIdentity` (Ed25519) work end-to-end
