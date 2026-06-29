// Phase 4.2 — Learned Decoder: neural network replacing Blake3 reverse-lookup.
//
// Architecture: small MLP that maps Ω tensor → Σ∞ packet.
//
// Input (29796-dim): Full OmegaPacket tensor
// Output: Opcode logits (16 dims, softmax → class probabilities)
//
// Behind the `learned` feature gate — when disabled, the Blake3 decoder
// from `decoder.rs::decompile` is used instead.

use a2x_core::Opcode;
use a2x_sigma::packet::SigmaPacket;

use crate::decoder::opcode_to_intent;
use crate::packet::{OmegaPacket, TOTAL_DIM};

use super::learned_encoder::{HIDDEN_DIM, NUM_OPCODES};

// ─── Learned Decoder ─────────────────────────────────────────────────

/// A simple 3-layer MLP decoder: TOTAL_DIM → HIDDEN_DIM → HIDDEN_DIM → NUM_OPCODES.
pub struct LearnedDecoder {
    pub(crate) w1: Vec<f32>,    // [TOTAL_DIM * HIDDEN_DIM]
    pub(crate) b1: Vec<f32>,    // [HIDDEN_DIM]
    pub(crate) w2: Vec<f32>,    // [HIDDEN_DIM * HIDDEN_DIM]
    pub(crate) b2: Vec<f32>,    // [HIDDEN_DIM]
    pub(crate) w3_op: Vec<f32>, // [HIDDEN_DIM * NUM_OPCODES]
    pub(crate) b3_op: Vec<f32>, // [NUM_OPCODES]
}

impl Default for LearnedDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LearnedDecoder {
    /// Create a new decoder with Xavier-initialized weights.
    pub fn new() -> Self {
        let mut rng_seed: u64 = 0xCAFE_BABE_DEAD_BEEF;
        let mut rand = || {
            rng_seed ^= rng_seed << 13;
            rng_seed ^= rng_seed >> 7;
            rng_seed ^= rng_seed << 17;
            (rng_seed as f64 / u64::MAX as f64) as f32
        };

        let w1 = super::learned_encoder::xavier_init(TOTAL_DIM, HIDDEN_DIM, &mut rand);
        let b1 = vec![0.0; HIDDEN_DIM];
        let w2 = super::learned_encoder::xavier_init(HIDDEN_DIM, HIDDEN_DIM, &mut rand);
        let b2 = vec![0.0; HIDDEN_DIM];
        let w3_op = super::learned_encoder::xavier_init(HIDDEN_DIM, NUM_OPCODES, &mut rand);
        let b3_op = vec![0.0; NUM_OPCODES];

        LearnedDecoder {
            w1,
            b1,
            w2,
            b2,
            w3_op,
            b3_op,
        }
    }

    /// Forward pass: OmegaPacket → opcode logits (pre-softmax).
    pub fn forward_raw(&self, packet: &OmegaPacket<TOTAL_DIM>) -> [f32; NUM_OPCODES] {
        let h1 = super::learned_encoder::linear_relu(
            &packet.data,
            &self.w1,
            &self.b1,
            TOTAL_DIM,
            HIDDEN_DIM,
        );
        let h2 =
            super::learned_encoder::linear_relu(&h1, &self.w2, &self.b2, HIDDEN_DIM, HIDDEN_DIM);
        let logits =
            super::learned_encoder::linear(&h2, &self.w3_op, &self.b3_op, HIDDEN_DIM, NUM_OPCODES);
        let mut out = [0.0f32; NUM_OPCODES];
        for (i, &v) in logits.iter().enumerate().take(NUM_OPCODES) {
            out[i] = v;
        }
        out
    }

    /// Decode an OmegaPacket to a SigmaPacket.
    pub fn decode(&self, packet: &OmegaPacket<TOTAL_DIM>) -> SigmaPacket {
        let logits = self.forward_raw(packet);
        let opcode = argmax_opcode(&logits);
        let mut pkt = SigmaPacket::new();
        if let Some(intent) = opcode_to_intent(opcode) {
            pkt.intent.operators.push(intent);
        }
        pkt
    }

    /// Update a single weight by flat index.
    pub fn set_weight(&mut self, flat_idx: usize, value: f32) {
        let mut idx = flat_idx;
        if idx < self.w1.len() {
            self.w1[idx] = value;
            return;
        }
        idx -= self.w1.len();
        if idx < self.b1.len() {
            self.b1[idx] = value;
            return;
        }
        idx -= self.b1.len();
        if idx < self.w2.len() {
            self.w2[idx] = value;
            return;
        }
        idx -= self.w2.len();
        if idx < self.b2.len() {
            self.b2[idx] = value;
            return;
        }
        idx -= self.b2.len();
        if idx < self.w3_op.len() {
            self.w3_op[idx] = value;
            return;
        }
        idx -= self.w3_op.len();
        self.b3_op[idx] = value;
    }

    /// Get weight at a specific flat index.
    pub fn get_weight(&self, flat_idx: usize) -> f32 {
        let mut idx = flat_idx;
        if idx < self.w1.len() {
            return self.w1[idx];
        }
        idx -= self.w1.len();
        if idx < self.b1.len() {
            return self.b1[idx];
        }
        idx -= self.b1.len();
        if idx < self.w2.len() {
            return self.w2[idx];
        }
        idx -= self.w2.len();
        if idx < self.b2.len() {
            return self.b2[idx];
        }
        idx -= self.b2.len();
        if idx < self.w3_op.len() {
            return self.w3_op[idx];
        }
        idx -= self.w3_op.len();
        self.b3_op[idx]
    }

    /// Total number of weights.
    pub fn num_weights(&self) -> usize {
        self.w1.len()
            + self.b1.len()
            + self.w2.len()
            + self.b2.len()
            + self.w3_op.len()
            + self.b3_op.len()
    }

    /// Get all weights as a flat vector (for serialization).
    pub fn weights_flat(&self) -> Vec<f32> {
        let mut w = Vec::with_capacity(self.num_weights());
        w.extend_from_slice(&self.w1);
        w.extend_from_slice(&self.b1);
        w.extend_from_slice(&self.w2);
        w.extend_from_slice(&self.b2);
        w.extend_from_slice(&self.w3_op);
        w.extend_from_slice(&self.b3_op);
        w
    }

    /// Load weights from a flat vector.
    pub fn load_weights_flat(&mut self, flat: &[f32]) -> Result<(), String> {
        let expected = self.num_weights();
        if flat.len() != expected {
            return Err(format!(
                "weight count mismatch: expected {}, got {}",
                expected,
                flat.len()
            ));
        }
        let mut off = 0;
        let take = |off: &mut usize, len: usize, flat: &[f32]| -> Vec<f32> {
            let v = flat[*off..*off + len].to_vec();
            *off += len;
            v
        };
        self.w1 = take(&mut off, self.w1.len(), flat);
        self.b1 = take(&mut off, self.b1.len(), flat);
        self.w2 = take(&mut off, self.w2.len(), flat);
        self.b2 = take(&mut off, self.b2.len(), flat);
        self.w3_op = take(&mut off, self.w3_op.len(), flat);
        self.b3_op = take(&mut off, self.b3_op.len(), flat);
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

/// Index of the maximum opcode logit → Opcode.
pub(crate) fn argmax_opcode(logits: &[f32; NUM_OPCODES]) -> Opcode {
    let (idx, _) = logits
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    match idx as u8 {
        0 => Opcode::Nop,
        1 => Opcode::Bind,
        2 => Opcode::Differentiate,
        3 => Opcode::Ground,
        4 => Opcode::Evolve,
        5 => Opcode::Reflect,
        6 => Opcode::Plan,
        7 => Opcode::Actuate,
        8 => Opcode::Jump,
        9 => Opcode::Branch,
        10 => Opcode::Call,
        11 => Opcode::Return,
        12 => Opcode::Fork,
        13 => Opcode::Merge,
        14 => Opcode::Halt,
        _ => Opcode::Custom(idx as u8),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argmax_opcode_known() {
        let mut logits = [0.0f32; NUM_OPCODES];
        logits[1] = 100.0; // Bind
        assert_eq!(argmax_opcode(&logits), Opcode::Bind);
    }

    #[test]
    fn test_learned_decoder_output_shape() {
        let decoder = LearnedDecoder::new();
        let packet: OmegaPacket<TOTAL_DIM> = OmegaPacket::zeros();
        let logits = decoder.forward_raw(&packet);
        assert_eq!(logits.len(), NUM_OPCODES);
    }

    #[test]
    fn test_learned_decoder_decode_produces_sigma_packet() {
        let decoder = LearnedDecoder::new();
        let packet: OmegaPacket<TOTAL_DIM> = OmegaPacket::zeros();
        let sigma = decoder.decode(&packet);
        assert!(sigma.intent.operators.len() <= 1);
    }

    #[test]
    fn test_learned_decoder_deterministic() {
        let decoder = LearnedDecoder::new();
        let packet: OmegaPacket<TOTAL_DIM> = OmegaPacket::zeros();
        let s1 = decoder.decode(&packet);
        let s2 = decoder.decode(&packet);
        assert_eq!(
            format!("{:?}", s1.intent.operators),
            format!("{:?}", s2.intent.operators)
        );
    }

    #[test]
    fn test_weights_roundtrip() {
        let decoder = LearnedDecoder::new();
        let original = decoder.weights_flat();
        let mut decoder2 = LearnedDecoder::new();
        decoder2.load_weights_flat(&original).unwrap();
        assert_eq!(decoder2.weights_flat(), original);
    }

    #[test]
    fn test_set_get_weight() {
        let mut decoder = LearnedDecoder::new();
        decoder.set_weight(0, 42.0);
        assert_eq!(decoder.get_weight(0), 42.0);
        let last = decoder.num_weights() - 1;
        decoder.set_weight(last, -7.0);
        assert_eq!(decoder.get_weight(last), -7.0);
    }
}
