# Phase 3.1 — Ω→Σ Decoder + Binary Wire Roundtrip

> **Date:** 2026-06-28
> **Status:** ✅ Complete — all tests green, clippy clean, fmt clean
> **Branch:** master (uncommitted)

---

## Summary

Completed Phase 3.1 of the Ω Latent Protocol: a real Ω→Σ∞ decoder, binary wire-format roundtrip, and all associated fixes.

## Changes

### 1. `crates/a2x-omega/src/packet.rs` — SIZE_D alignment fix
- Changed `SIZE_D` from `16384` to `16484`
- Root cause: `OFFSET_D + SIZE_D` must equal the struct's default generic param `29796`. With `OFFSET_D = 13312` and `SIZE_D = 16384`, `TOTAL_DIM` evaluated to `29696` — 100 short of `29796`. The data region is actually `29796 - 13312 = 16484` elements.
- This fixed 4 failing wire roundtrip integration tests that used `TOTAL_DIM` for size calculations.

### 2. `crates/a2x-omega/src/decoder.rs` — Real Ω→Σ∞ decoder (Phase 3.1)
- Rewrote from stub to full implementation (~270 LOC, 12 unit tests)
- Builds a `HashMap<[u8; 32], IntentOp>` reverse-lookup table from Blake3(opcode) hashes
- Decodes Ω→Σ by reading the first 32 f32 slots of `intent_slice()`, clamping to [0,1], rescaling ×255→u8
- Supports all 10 IntentOp-mapped standard opcodes; control-flow opcodes return `DecompileError::NoMatchingOperator`
- Refactored `intent_hash_table()` to build from `opcode_to_intent()` — eliminates mapping duplication and dead_code warning
- Fixed clippy: `.clone()` → `*` deref on `[u8; 32]` Copy types, identity_op (`& 0xFF`), needless_range_loop

### 3. `crates/a2x-omega/src/bridge.rs` — Bridge test updated for Phase 3.1
- Updated `test_bridge_compile_decompile` from asserting `None` (Phase 0 stub) to asserting `Some(Lightning)` (real decoder)
- Added `test_bridge_decompile_unmapped_returns_none` for negative path coverage

### 4. `crates/a2x-omega/tests/omega_wire_roundtrip.rs` — Wire format integration tests (7 tests)
- Proves `OmegaProgram<29796>` encodes to deterministic binary framing and decodes back byte-identically
- Wire format: `[4-byte BE length prefix][data.len() × 4-byte LE f32 payload]`
- Fixed `test_single_packet_roundtrip_byte_identical`: removed metadata/source_id assertions (wire format doesn't preserve them)
- Fixed `test_frame_boundaries_preserved_under_repeated_packets`: compare position-wise instead of cross-position (compiler encodes source_index into data region)

## Verification

| Check | Result |
|-------|--------|
| `cargo test -p a2x-omega` | ✅ 31 passed (24 unit + 7 integration) |
| `cargo clippy -p a2x-omega --all-targets -- -D warnings` | ✅ Clean |
| `cargo fmt -p a2x-omega` | ✅ Clean |
| `cargo test -p a2x-core -p a2x-ccs -p a2x-agents` | ✅ All green |
| Code review | ✅ "Ship it." |

## Next Steps

1. **Phase 3.2**: Implement 4 IR optimizer passes (constant folding, dead code elimination, instruction fusion, layout optimization)
2. **Phase 3.3**: TcpTransport in `a2x-bus/src/tcp_transport.rs` (sync std::net, length-prefix framing)
3. **Phase 3.4**: Cross-machine Σ→Ω→CCS end-to-end demo via TcpTransport
