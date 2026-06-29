# Phase 3.2+3.3 — IR Optimizer Passes + TcpTransport

> **Date:** 2026-06-28
> **Status:** ✅ Complete — all tests green, clippy clean, fmt clean

---

## Phase 3.2: IR Optimizer Passes

### Changes
- **`ir.rs`**: Added `fused: bool` to `IrMetadata` (derives Default → false)
- **`compiler.rs`**: Added `fused: false` to `IrMetadata` initializer
- **`passes/constant_folding.rs`**: Real impl — folds `Bind` nodes with all-Immediate operands into `Nop` with merged immediate (5 tests)
- **`passes/dead_code.rs`**: Real impl — removes nodes not referenced in any `control_flow` edge, with flat-sequential early-return guard (7 tests)
- **`passes/fusion.rs`**: Real impl — merges adjacent `Bind+Differentiate` pairs with matching labels, marks fused (7 tests)
- **`passes/layout.rs`**: Real impl — sorts by `source_index` for cache locality, asserts idempotence (5 tests)

### Key fix: dead_code elimination
The initial implementation removed ALL nodes from compiled programs because `build_ir()` never sets `control_flow` or `entry`/`exit`. Fixed by adding an early-return when the graph has no `entry`/`exit` and no `control_flow` edges (flat sequential program → all nodes are live).

---

## Phase 3.3: TcpTransport

### Changes
- **`tcp_transport.rs`** (new, ~350 LOC): Sync `std::net` TCP transport with manual binary framing
- **`lib.rs`**: Added `tcp_transport` module + `TcpTransport` re-export
- Wire format: `[4-byte BE length][body]` where body is field-by-field binary (version, tag, sender, recipient, correlation_id, timestamp, payload)
- `encode_payload_for_type()` serializes enum-variant data (ProgramRequest ID, Error code/message) into payload for wire transmission
- `Mutex`-wrapped listeners for `Send + Sync` compliance
- 10 unit tests: codec roundtrip, loopback send/recv, framing boundaries, ephemeral port, recv drain

---

## Verification

| Check | Result |
|-------|--------|
| `cargo test -p a2x-omega` | ✅ 53 passed (46 unit + 7 integration) |
| `cargo test -p a2x-bus` | ✅ 20 passed |
| `cargo clippy --all-targets -- -D warnings` | ✅ Clean (both crates) |
| `cargo fmt` | ✅ Clean |
| Code review | ✅ "Ship it." |
