# Ω Learned Encoders

Phase 4 introduces neural network-based encoder/decoder models that replace the
deterministic Blake3 projections with learned representations.

## Why Learned Encoding?

The deterministic encoding has limitations:
- All operators are equidistant in latent space (no semantic similarity)
- Concept vectors don't capture contextual meaning
- No transfer learning between related operators

Learned encoders address these by training on execution traces.

## Architecture

### Learned Encoder

```
Σ∞ Token → Embedding → Transformer → 29,796-dim Ω tensor
```

### Learned Decoder

```
Ω tensor → Transformer → Decoder → Σ∞ Token
```

## Training

```rust
use a2x_omega::{LearnedEncoder, LearnedDecoder, TrainingConfig};

let config = TrainingConfig {
    epochs: 100,
    batch_size: 32,
    learning_rate: 1e-4,
};

let mut encoder = LearnedEncoder::new()?;
let mut decoder = LearnedDecoder::new()?;

// Train on a corpus of Σ∞ programs
encoder.train(&training_data, &config)?;
decoder.train(&training_data, &config)?;
```

## Candle Integration

Training uses the `candle` framework for GPU-accelerated computation:

- CUDA support (NVIDIA GPUs)
- Metal support (Apple Silicon)
- CPU fallback

## Performance

| Encoding | Latency | Semantic Similarity |
|----------|---------|---------------------|
| Blake3 (deterministic) | ~50µs | No |
| Learned (GPU) | ~5ms | Yes |
| Learned (CPU) | ~20ms | Yes |

## Current Status

The learned encoder is experimental (Phase 4). The deterministic encoding
remains the default for v0.9.0-alpha.
