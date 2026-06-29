// Phase 4.1 — Learned Encoder: neural network replacing Blake3 hash projection.
//
// Architecture: small MLP that maps IR node features → Ω tensor.
//
// Input (144-dim):
//   - Opcode one-hot embedding: 16 dims (one per Opcode variant)
//   - Operand features: 64 dims (Blake3 of operand labels, pooled)
//   - Control-flow features: 64 dims (Blake3 of target IDs, pooled)
//
// Output (29796-dim):
//   - OmegaPacket tensor, tanh-activated (values in [-1, 1])
//
// The encoder is trained end-to-end with the learned decoder (4.2) using
// a roundtrip loss: encode(source) → decode(Ω) ≈ source.
//
// Behind the `learned` feature gate — when disabled, the Blake3 encoder
// from `compiler.rs::encode_instruction` is used instead.

use crate::ir::IrNode;
use crate::packet::{OmegaPacket, TOTAL_DIM};
use a2x_core::Opcode;

/// Total feature dimension fed to the encoder MLP.
pub(crate) const INPUT_DIM: usize = 144;

/// Hidden dimension for encoder/decoder MLP layers.
pub(crate) const HIDDEN_DIM: usize = 256;

/// Number of recognized opcodes (0x0..=0xE = 15, plus Nop at 0x0).
pub(crate) const NUM_OPCODES: usize = 16;

/// Number of operand/control-flow feature dims.
pub(crate) const FEATURE_DIM: usize = 64;

// ─── Feature extraction ──────────────────────────────────────────────

/// Produce a 16-dim one-hot vector for the opcode.
pub(crate) fn opcode_one_hot(op: Opcode) -> [f32; NUM_OPCODES] {
    let mut v = [0.0f32; NUM_OPCODES];
    let idx = op.as_u8() as usize;
    if idx < NUM_OPCODES {
        v[idx] = 1.0;
    }
    v
}

/// Hash a byte slice into a fixed-size feature vector (64 f32s).
pub(crate) fn hash_to_features(data: &[u8]) -> [f32; FEATURE_DIM] {
    let hash = blake3::hash(data);
    let bytes = hash.as_bytes();
    let mut features = [0.0f32; FEATURE_DIM];
    for i in 0..FEATURE_DIM.min(bytes.len() / 2) {
        let b0 = bytes[i * 2] as f32;
        let b1 = bytes[i * 2 + 1] as f32;
        features[i] = (b0 * 256.0 + b1) / 65535.0;
    }
    features
}

/// Extract a 144-dim feature vector from an IR node.
pub(crate) fn extract_features(node: &IrNode) -> [f32; INPUT_DIM] {
    let mut features = [0.0f32; INPUT_DIM];
    let mut offset = 0;

    // 1. Opcode one-hot (16 dims)
    let oh = opcode_one_hot(node.opcode);
    features[offset..offset + NUM_OPCODES].copy_from_slice(&oh);
    offset += NUM_OPCODES;

    // 2. Operand features (64 dims)
    let mut operand_bytes = Vec::new();
    for op in &node.operands {
        match op {
            crate::ir::IrOperand::Label(s) => operand_bytes.extend_from_slice(s.as_bytes()),
            crate::ir::IrOperand::NodeId(id) => operand_bytes.extend_from_slice(&id.to_le_bytes()),
            crate::ir::IrOperand::Region(s) => {
                operand_bytes.extend_from_slice(s.as_bytes());
                operand_bytes.push(0xFF); // separator
            }
            crate::ir::IrOperand::Immediate(b) => operand_bytes.extend_from_slice(b),
        }
    }
    let operand_feats = if operand_bytes.is_empty() {
        [0.0f32; FEATURE_DIM]
    } else {
        hash_to_features(&operand_bytes)
    };
    features[offset..offset + FEATURE_DIM].copy_from_slice(&operand_feats);
    offset += FEATURE_DIM;

    // 3. Control-flow features (64 dims)
    let mut cf_bytes = Vec::new();
    for target in &node.control_flow {
        cf_bytes.extend_from_slice(&target.0.to_le_bytes());
    }
    let cf_feats = if cf_bytes.is_empty() {
        [0.0f32; FEATURE_DIM]
    } else {
        hash_to_features(&cf_bytes)
    };
    features[offset..offset + FEATURE_DIM].copy_from_slice(&cf_feats);

    features
}

// ─── Learned Encoder ─────────────────────────────────────────────────

/// A simple 3-layer MLP encoder: INPUT_DIM → HIDDEN_DIM → HIDDEN_DIM → TOTAL_DIM.
pub struct LearnedEncoder {
    pub(crate) w1: Vec<f32>, // [INPUT_DIM * HIDDEN_DIM]
    pub(crate) b1: Vec<f32>, // [HIDDEN_DIM]
    pub(crate) w2: Vec<f32>, // [HIDDEN_DIM * HIDDEN_DIM]
    pub(crate) b2: Vec<f32>, // [HIDDEN_DIM]
    pub(crate) w3: Vec<f32>, // [HIDDEN_DIM * TOTAL_DIM]
    pub(crate) b3: Vec<f32>, // [TOTAL_DIM]
}

impl Default for LearnedEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LearnedEncoder {
    /// Create a new encoder with Xavier-initialized weights.
    pub fn new() -> Self {
        let mut rng_seed: u64 = 0xDEAD_BEEF_CAFE_BABE;
        let mut rand = || {
            rng_seed ^= rng_seed << 13;
            rng_seed ^= rng_seed >> 7;
            rng_seed ^= rng_seed << 17;
            (rng_seed as f64 / u64::MAX as f64) as f32
        };

        let w1 = xavier_init(INPUT_DIM, HIDDEN_DIM, &mut rand);
        let b1 = vec![0.0; HIDDEN_DIM];
        let w2 = xavier_init(HIDDEN_DIM, HIDDEN_DIM, &mut rand);
        let b2 = vec![0.0; HIDDEN_DIM];
        let w3 = xavier_init(HIDDEN_DIM, TOTAL_DIM, &mut rand);
        let b3 = vec![0.0; TOTAL_DIM];

        LearnedEncoder {
            w1,
            b1,
            w2,
            b2,
            w3,
            b3,
        }
    }

    /// Forward pass: features → OmegaPacket.
    pub fn forward(&self, features: &[f32; INPUT_DIM]) -> OmegaPacket<TOTAL_DIM> {
        let h1 = linear_relu(features, &self.w1, &self.b1, INPUT_DIM, HIDDEN_DIM);
        let h2 = linear_relu(&h1, &self.w2, &self.b2, HIDDEN_DIM, HIDDEN_DIM);
        let raw = linear(&h2, &self.w3, &self.b3, HIDDEN_DIM, TOTAL_DIM);
        let mut data = [0.0f32; TOTAL_DIM];
        for (i, v) in raw.iter().enumerate().take(TOTAL_DIM) {
            data[i] = v.tanh();
        }
        OmegaPacket::from_raw(data)
    }

    /// Encode an IR node directly.
    pub fn encode_node(&self, node: &IrNode) -> OmegaPacket<TOTAL_DIM> {
        let features = extract_features(node);
        self.forward(&features)
    }

    /// Update weight at a specific flat index.
    pub fn set_weight(&mut self, flat_idx: usize, value: f32) {
        let total = self.w1.len()
            + self.b1.len()
            + self.w2.len()
            + self.b2.len()
            + self.w3.len()
            + self.b3.len();
        assert!(flat_idx < total);
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
        if idx < self.w3.len() {
            self.w3[idx] = value;
            return;
        }
        idx -= self.w3.len();
        self.b3[idx] = value;
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
        if idx < self.w3.len() {
            return self.w3[idx];
        }
        idx -= self.w3.len();
        self.b3[idx]
    }

    /// Total number of weights.
    pub fn num_weights(&self) -> usize {
        self.w1.len()
            + self.b1.len()
            + self.w2.len()
            + self.b2.len()
            + self.w3.len()
            + self.b3.len()
    }

    /// Get all weights as a flat vector (for serialization).
    pub fn weights_flat(&self) -> Vec<f32> {
        let mut w = Vec::with_capacity(self.num_weights());
        w.extend_from_slice(&self.w1);
        w.extend_from_slice(&self.b1);
        w.extend_from_slice(&self.w2);
        w.extend_from_slice(&self.b2);
        w.extend_from_slice(&self.w3);
        w.extend_from_slice(&self.b3);
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
        self.w3 = take(&mut off, self.w3.len(), flat);
        self.b3 = take(&mut off, self.b3.len(), flat);
        Ok(())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────

pub(crate) fn xavier_init<F: FnMut() -> f32>(
    fan_in: usize,
    fan_out: usize,
    rng: &mut F,
) -> Vec<f32> {
    let limit = (6.0 / (fan_in + fan_out) as f32).sqrt();
    (0..fan_in * fan_out)
        .map(|_| rng() * 2.0 * limit - limit)
        .collect()
}

pub(crate) fn linear_relu(
    x: &[f32],
    w: &[f32],
    b: &[f32],
    in_dim: usize,
    out_dim: usize,
) -> Vec<f32> {
    (0..out_dim)
        .map(|o| {
            let sum: f32 = (0..in_dim).map(|i| x[i] * w[o * in_dim + i]).sum();
            (sum + b[o]).max(0.0)
        })
        .collect()
}

pub(crate) fn linear(x: &[f32], w: &[f32], b: &[f32], in_dim: usize, out_dim: usize) -> Vec<f32> {
    (0..out_dim)
        .map(|o| {
            let sum: f32 = (0..in_dim).map(|i| x[i] * w[o * in_dim + i]).sum();
            sum + b[o]
        })
        .collect()
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrMetadata, IrNodeId};

    #[test]
    fn test_opcode_one_hot_all_variants() {
        let opcodes = [
            Opcode::Nop,
            Opcode::Bind,
            Opcode::Differentiate,
            Opcode::Ground,
            Opcode::Evolve,
            Opcode::Reflect,
            Opcode::Plan,
            Opcode::Actuate,
            Opcode::Jump,
            Opcode::Branch,
            Opcode::Call,
            Opcode::Return,
            Opcode::Fork,
            Opcode::Merge,
            Opcode::Halt,
        ];
        for op in opcodes {
            let oh = opcode_one_hot(op);
            assert_eq!(oh.iter().filter(|&&x| x > 0.5).count(), 1, "{:?}", op);
        }
    }

    #[test]
    fn test_hash_to_features_deterministic() {
        let f1 = hash_to_features(b"hello");
        let f2 = hash_to_features(b"hello");
        assert_eq!(f1, f2);
    }

    #[test]
    fn test_hash_to_features_different_inputs() {
        let f1 = hash_to_features(b"hello");
        let f2 = hash_to_features(b"world");
        assert_ne!(f1, f2);
    }

    #[test]
    fn test_extract_features_total_dim() {
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Bind,
            operands: vec![crate::ir::IrOperand::Label("a".into())],
            control_flow: vec![IrNodeId(1)],
            metadata: IrMetadata::default(),
        };
        let features = extract_features(&node);
        assert_eq!(features.len(), INPUT_DIM);
    }

    #[test]
    fn test_extract_features_opcode_position() {
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Bind,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let features = extract_features(&node);
        assert_eq!(features[1], 1.0);
        assert_eq!(features[0], 0.0);
    }

    #[test]
    fn test_learned_encoder_output_shape() {
        let encoder = LearnedEncoder::new();
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Ground,
            operands: vec![crate::ir::IrOperand::Label("sys".into())],
            control_flow: vec![IrNodeId(1)],
            metadata: IrMetadata::default(),
        };
        let omega = encoder.encode_node(&node);
        assert_eq!(omega.data.len(), TOTAL_DIM);
    }

    #[test]
    fn test_learned_encoder_output_range() {
        let encoder = LearnedEncoder::new();
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Plan,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let omega = encoder.encode_node(&node);
        for &v in omega.data.iter() {
            assert!((-1.0..=1.0).contains(&v), "output value {} out of range", v);
        }
    }

    #[test]
    fn test_learned_encoder_deterministic() {
        let encoder = LearnedEncoder::new();
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Nop,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let o1 = encoder.encode_node(&node);
        let o2 = encoder.encode_node(&node);
        assert_eq!(o1.data, o2.data);
    }

    #[test]
    fn test_learned_encoder_different_opcodes_differ() {
        let encoder = LearnedEncoder::new();
        let n1 = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Bind,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let n2 = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Ground,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let o1 = encoder.encode_node(&n1);
        let o2 = encoder.encode_node(&n2);
        assert_ne!(o1.data[..16], o2.data[..16]);
    }

    #[test]
    fn test_weights_roundtrip() {
        let encoder = LearnedEncoder::new();
        let original = encoder.weights_flat();
        let mut encoder2 = LearnedEncoder::new();
        encoder2.load_weights_flat(&original).unwrap();
        assert_eq!(encoder2.weights_flat(), original);
    }

    #[test]
    fn test_set_get_weight() {
        let mut encoder = LearnedEncoder::new();
        encoder.set_weight(0, 42.0);
        assert_eq!(encoder.get_weight(0), 42.0);
        let last = encoder.num_weights() - 1;
        encoder.set_weight(last, -7.0);
        assert_eq!(encoder.get_weight(last), -7.0);
    }
}
