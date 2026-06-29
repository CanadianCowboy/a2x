# Phase 3.4 — Cross-Machine Σ↔Ω↔CCS End-to-End Demo

> **Date:** 2026-06-28
> **Crate:** `a2x-agents` (integration test)
> **Checkpoint ref:** §4.4

---

## 1. What was delivered

A single integration test (`crates/a2x-agents/tests/phase3_cross_machine.rs`) that exercises the full cross-machine pipeline:

1. **Sender parses Σ∞** — `⟦Σ∞⟧⟬I:✦ ∷ C:⟨⟩ ∷ P:⥁ ∷ D:⌬⟭` (Ground intent, Star operator).
2. **Compiles to Ω** — `CompileToOmega::compile(OptimizationLevel::Light)`.
3. **Computes expected round-trip** — local decompile of the Ω packet to determine the expected Σ∞ form (Ω encoder only preserves intent hashes; context/plan/data fields are not round-tripped in Phase 3).
4. **Serializes Ω packet** — length-prefixed wire bytes (4-byte BE length + 29,796 × 4-byte LE f32).
5. **TCP transport (sender → receiver)** — `TcpTransport` over loopback.
6. **Receiver reconstructs Ω packet** from wire bytes.
7. **Decompiles Ω → Σ∞** — `DecompileToSigma::decompile`, asserts `IntentOp::Star` survived.
8. **CCS VM execution** — loads the decompiled Σ∞ into a fresh `CcsVm`, ticks 1 step, asserts `Ground` allocates exactly 1 `WorldGraph` node and `steps_executed == 1`.
9. **Sends result back (receiver → sender)** — serializes the decompiled Σ∞ as length-prefixed UTF-8 wire bytes, sends via a second `TcpTransport` pair.
10. **Identity check** — sender asserts the received result string matches the expected round-tripped Σ∞ string.

### Helper functions extracted

| Function | Purpose |
|----------|---------|
| `serialize_omega_packet` | `OmegaPacket → Vec<u8>` (length-prefixed f32 LE) |
| `deserialize_omega_packet` | `&[u8] → OmegaPacket` |
| `serialize_sigma_packet` | `SigmaPacket → Vec<u8>` (length-prefixed UTF-8) |

---

## 2. Files touched

| File | Change |
|------|--------|
| `crates/a2x-agents/tests/phase3_cross_machine.rs` | Rewritten: full §4.4 pipeline with CCS VM tick + round-trip identity check |
| `crates/a2x-agents/Cargo.toml` | Added `a2x-omega` as dev-dependency (Phase 3.2 work) |

---

## 3. Validation

| Check | Result |
|-------|--------|
| `cargo test -p a2x-agents --test phase3_cross_machine` | ✅ 1 passed |
| `cargo clippy -p a2x-agents --all-targets -- -D warnings` | ✅ Clean |
| `cargo fmt -p a2x-agents` | ✅ Clean |

---

## 4. Design notes

- **Two separate TcpTransport pairs** are used (one per direction) because `TcpTransport` doesn't support bidirectional communication on a single listener. This matches the checkpoint spec: "Two paired TcpStreams."
- **Identity check uses `expected_roundtripped`** rather than the original source Σ∞. The Ω encoder projects only the intent hash into Ω_I; context/plan/data fields are lost in compile→decompile. The decompiled form is the ground truth for round-trip identity.
- **CCS VM tick on Ground** is the simplest meaningful execution: Ground allocates exactly 1 WorldGraph node with provenance `ground(ip=0,modality=Text,floats=0)`. This proves the decompiled Σ∞ is valid input to the CCS VM execution loop.
- **No new dependencies** added for this test — `a2x-omega` was already a dev-dependency from Phase 3.2.

---

*This file is part of the A2X project. See `PLAN.md` for the full architecture.*
