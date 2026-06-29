# Phase 4 — Learned Encoder/Decoder, Training Loop, Simulated Environment

**Date:** 2026-06-28  
**Tag:** v0.4.0 (commit `584b65d`)  
**Scope:** a2x-omega — neural-network encoder/decoder replacing Blake3 hash projection

---

## 4.1 Learned Encoder (`learned_encoder.rs`)

**Architecture:** 3-layer MLP mapping IR node features → Ω tensor.

| Layer | Input → Output | Activation |
|-------|---------------|------------|
| fc1 | 144 → 256 | ReLU |
| fc2 | 256 → 256 | ReLU |
| fc3 | 256 → 29796 | tanh |

**Feature extraction (144-dim input):**
- Opcode one-hot: 16 dims (one per Opcode variant)
- Operand features: 64 dims (Blake3 of operand labels, pooled to f32s)
- Control-flow features: 64 dims (Blake3 of target node IDs, pooled)

**Key design decisions:**
- Xavier initialization for all weight matrices
- tanh output ensures values in [-1, 1] matching OmegaPacket range
- Index-based weight access (`set_weight`/`get_weight`/`num_weights`) avoids borrow conflicts with `forward()`
- `Default` trait implemented delegating to `new()`
- `weights_flat()`/`load_weights_flat()` for serialization roundtrip

**Tests:** 11 unit tests (shape, range, determinism, different opcodes, weight roundtrip, set/get)

## 4.2 Learned Decoder (`learned_decoder.rs`)

**Architecture:** 3-layer MLP mapping Ω tensor → opcode logits.

| Layer | Input → Output | Activation |
|-------|---------------|------------|
| fc1 | 29796 → 256 | ReLU |
| fc2 | 256 → 256 | ReLU |
| fc3 | 256 → 16 | none (raw logits) |

**Key design decisions:**
- `argmax_opcode()` with full `Opcode` match including `Custom(u8)` variant
- `decode()` maps predicted opcode → IntentOp via `opcode_to_intent()` (made `pub(crate)`)
- Same index-based weight API as encoder for consistency

**Tests:** 6 unit tests (argmax, shape, decode output, determinism, weight roundtrip, set/get)

## 4.3 Training Loop (`training.rs`)

**Approach:** Perturbation-based evolutionary training (no autodiff).

**Loss functions:**
- **Encoder loss:** MSE between learned Ω output and Blake3 Ω (teacher signal)
- **Decoder loss:** Cross-entropy on 16-class opcode classification

**Perturbation update:**
- Sample `num_weights / 16` random weights per training example
- Perturb by `±eps` (scaled by learning_rate), keep if loss improves
- Uses `PerturbationParams` struct with references to avoid 120KB copy per call
- Unsigned modulo for uniform weight sampling across full range

**Tests:** 7 unit tests (cross-entropy, MSE, intent mapping, Blake3 encoding)

## 4.4 Simulated Environment (`environment.rs`)

**Purpose:** Generate synthetic Σ∞ programs for training data.

- Deterministic PRNG (xorshift) for reproducible batches
- Random intent operators from the 10 standard IntentOps
- Random context labels, plan operators, data payloads
- `saturating_sub` overflow guard on min/max packet counts

**Tests:** 5 unit tests (nonempty, deterministic, different seeds, batch size, has intent)

## Integration

- All 4 modules behind `cfg(feature = "learned")` cargo feature gate
- `learned` feature in `Cargo.toml` (candle deps removed as dead weight — pure-Rust MLP)
- `decoder.rs` — `opcode_to_intent()` made `pub(crate)` for learned decoder access
- Workspace `Cargo.toml` cleaned of unused candle dependencies

## Validation

- ✅ 83 tests pass (76 a2x-omega + 7 wire roundtrip)
- ✅ `cargo clippy -p a2x-omega --features learned --all-targets -- -D warnings` clean
- ✅ `cargo fmt --all` clean
- ✅ Full workspace test suite passes
