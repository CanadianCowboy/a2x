// Phase 4.3 — Training Loop: encode→decode roundtrip training.
//
// The training procedure:
// 1. Generate random Σ∞ programs using the simulated environment
// 2. Compile each packet to Ω using the Blake3 encoder (teacher signal)
// 3. Train the learned encoder to produce Ω tensors that the learned
//    decoder can reconstruct back to the correct opcode
//
// Behind the `learned` feature gate.

use crate::environment::generate_batch;
use crate::ir::{IrMetadata, IrNode, IrNodeId, IrOperand};
use crate::learned_decoder::LearnedDecoder;
use crate::learned_encoder::LearnedEncoder;
use crate::packet::{OmegaPacket, TOTAL_DIM};
use a2x_core::Opcode;
use a2x_sigma::intent::IntentOp;

/// Training configuration.
#[derive(Clone, Debug)]
pub struct TrainConfig {
    pub epochs: usize,
    pub batch_size: usize,
    pub learning_rate: f32,
    pub min_packets: usize,
    pub max_packets: usize,
    pub seed: u64,
}

impl Default for TrainConfig {
    fn default() -> Self {
        TrainConfig {
            epochs: 100,
            batch_size: 32,
            learning_rate: 0.001,
            min_packets: 1,
            max_packets: 4,
            seed: 42,
        }
    }
}

/// Training metrics for one epoch.
#[derive(Clone, Debug, Default)]
pub struct EpochMetrics {
    pub encoder_loss: f32,
    pub decoder_loss: f32,
    pub accuracy: f32,
}

/// Map an IntentOp to the Opcode the Blake3 encoder would produce.
fn intent_to_opcode(intent: IntentOp) -> Opcode {
    match intent {
        IntentOp::Synthesis => Opcode::Bind,
        IntentOp::Split => Opcode::Differentiate,
        IntentOp::Star => Opcode::Ground,
        IntentOp::Delay => Opcode::Evolve,
        IntentOp::Contradiction => Opcode::Reflect,
        IntentOp::Lightning => Opcode::Plan,
        IntentOp::Warning => Opcode::Actuate,
        IntentOp::Parallel => Opcode::Fork,
        IntentOp::Merge => Opcode::Merge,
        IntentOp::Cancel => Opcode::Halt,
        _ => Opcode::Nop,
    }
}

/// Map an IntentOp to its index in the 16-dim opcode vector.
fn intent_to_index(intent: IntentOp) -> usize {
    intent_to_opcode(intent).as_u8() as usize
}

/// Build an IrNode from a SigmaPacket for encoding.
fn packet_to_ir_node(packet: &a2x_sigma::packet::SigmaPacket, index: usize) -> IrNode {
    let opcode = if packet.intent.is_empty() {
        Opcode::Nop
    } else {
        intent_to_opcode(packet.intent.operators[0])
    };

    let operands: Vec<IrOperand> = packet
        .context
        .labels
        .iter()
        .map(|l| IrOperand::Label(l.clone()))
        .collect();

    IrNode {
        id: IrNodeId(index as u32),
        opcode,
        operands,
        control_flow: Vec::new(),
        metadata: IrMetadata {
            source_index: Some(index),
            source_position: Some(index),
            fused: false,
        },
    }
}

/// Compute cross-entropy loss for a single opcode prediction.
fn cross_entropy_loss(predicted_logits: &[f32; 16], target_index: usize) -> f32 {
    let max = predicted_logits
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = predicted_logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    -(exps[target_index] / sum).max(1e-7).ln()
}

/// Compute MSE between two f32 slices.
fn mse_loss(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len()) as f32;
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        / n
}

/// Run the training loop. Returns the final metrics per epoch.
pub fn train(config: &TrainConfig) -> Vec<EpochMetrics> {
    let mut encoder = LearnedEncoder::new();
    let mut decoder = LearnedDecoder::new();
    let mut all_metrics = Vec::with_capacity(config.epochs);

    for epoch in 0..config.epochs {
        let programs = generate_batch(
            config.seed + epoch as u64,
            config.batch_size,
            config.min_packets,
            config.max_packets,
        );

        let mut total_encoder_loss = 0.0f32;
        let mut total_decoder_loss = 0.0f32;
        let mut correct = 0usize;
        let mut total = 0usize;

        for program in &programs {
            for (i, packet) in program.instructions.iter().enumerate() {
                if packet.intent.is_empty() {
                    continue;
                }

                // 1. Build IR node and encode with Blake3 (teacher)
                let ir_node = packet_to_ir_node(packet, i);
                let blake3_omega = encode_with_blake3(&ir_node);

                // 2. Extract features and encode with learned encoder
                let features = crate::learned_encoder::extract_features(&ir_node);
                let learned_omega = encoder.forward(&features);

                // 3. Encoder loss: MSE between learned and Blake3 Ω
                let enc_loss = mse_loss(&learned_omega.data, &blake3_omega.data);
                total_encoder_loss += enc_loss;

                // 4. Decode with learned decoder
                let decoder_logits = decoder.forward_raw(&learned_omega);

                // 5. Decoder loss: cross-entropy on opcode
                let target_idx = intent_to_index(packet.intent.operators[0]);
                let dec_loss = cross_entropy_loss(&decoder_logits, target_idx);
                total_decoder_loss += dec_loss;

                // 6. Check accuracy
                let predicted_idx = decoder_logits
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                if predicted_idx == target_idx {
                    correct += 1;
                }
                total += 1;

                // 7. Simplified gradient update (perturbation-based)
                let params = PerturbationParams {
                    features: &features,
                    target_omega: &blake3_omega.data,
                    target_opcode: target_idx,
                    current_enc_loss: enc_loss,
                    current_dec_loss: dec_loss,
                    learning_rate: config.learning_rate,
                };
                apply_perturbation_update(&mut encoder, &mut decoder, &params);
            }
        }

        let n = total.max(1) as f32;
        let metrics = EpochMetrics {
            encoder_loss: total_encoder_loss / n,
            decoder_loss: total_decoder_loss / n,
            accuracy: correct as f32 / n,
        };
        all_metrics.push(metrics);
    }

    all_metrics
}

/// Blake3-based encoding (teacher signal) — mirrors compiler.rs::encode_instruction.
fn encode_with_blake3(node: &IrNode) -> OmegaPacket<TOTAL_DIM> {
    let mut packet = OmegaPacket::<TOTAL_DIM>::zeros();
    let hash = blake3::hash(&[node.opcode.as_u8()]);
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(32) {
        packet.intent_slice_mut()[j] = byte as f32 / 255.0;
    }
    packet
}

/// Parameters for perturbation-based gradient update (avoids too-many-arguments).
struct PerturbationParams<'a> {
    features: &'a [f32; 144],
    target_omega: &'a [f32; TOTAL_DIM],
    target_opcode: usize,
    current_enc_loss: f32,
    current_dec_loss: f32,
    learning_rate: f32,
}

/// Simplified perturbation-based gradient update.
///
/// For each weight, we perturb it, re-encode/re-decode, and keep the
/// change if loss improved. Uses index-based access to avoid borrow conflicts.
fn apply_perturbation_update(
    encoder: &mut LearnedEncoder,
    decoder: &mut LearnedDecoder,
    params: &PerturbationParams<'_>,
) {
    let eps = 1e-4 * params.learning_rate.clamp(0.0001, 1.0);
    let mut rng_state: u64 = 0xBEEF_CAFE ^ (params.target_opcode as u64) << 32;
    let mut xorshift_u64 = || -> u64 {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    let enc_num = encoder.num_weights();
    let sample_enc = (enc_num / 16).max(1);
    for _ in 0..sample_enc {
        let idx = (xorshift_u64() as usize) % enc_num;
        let old = encoder.get_weight(idx);
        let delta = eps * if xorshift_u64() % 2 == 0 { 1.0 } else { -1.0 };
        encoder.set_weight(idx, old + delta);

        let omega = encoder.forward(params.features);
        let new_loss = mse_loss(&omega.data, params.target_omega);
        if new_loss >= params.current_enc_loss {
            encoder.set_weight(idx, old); // revert
        }
    }

    let dec_num = decoder.num_weights();
    let sample_dec = (dec_num / 16).max(1);
    for _ in 0..sample_dec {
        let idx = (xorshift_u64() as usize) % dec_num;
        let old = decoder.get_weight(idx);
        let delta = eps * if xorshift_u64() % 2 == 0 { 1.0 } else { -1.0 };
        decoder.set_weight(idx, old + delta);

        let omega = encoder.forward(params.features);
        let logits = decoder.forward_raw(&omega);
        let predicted = logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        if predicted != params.target_opcode && params.current_dec_loss < 2.0 {
            decoder.set_weight(idx, old); // revert
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_train_runs_without_panic() {
        let config = TrainConfig {
            epochs: 2,
            batch_size: 4,
            ..Default::default()
        };
        let metrics = train(&config);
        assert_eq!(metrics.len(), 2);
        for m in &metrics {
            assert!(m.encoder_loss.is_finite());
            assert!(m.decoder_loss.is_finite());
            assert!(m.accuracy >= 0.0 && m.accuracy <= 1.0);
        }
    }

    #[test]
    fn test_cross_entropy_perfect_prediction() {
        let mut logits = [0.0f32; 16];
        logits[3] = 100.0;
        let loss = cross_entropy_loss(&logits, 3);
        assert!(loss < 0.01, "perfect prediction loss = {}", loss);
    }

    #[test]
    fn test_cross_entropy_wrong_prediction() {
        let mut logits = [0.0f32; 16];
        logits[3] = 100.0;
        let loss_correct = cross_entropy_loss(&logits, 3);
        let loss_wrong = cross_entropy_loss(&logits, 5);
        assert!(loss_wrong > loss_correct);
    }

    #[test]
    fn test_mse_identical() {
        let a = [1.0f32; 100];
        let b = [1.0f32; 100];
        assert_eq!(mse_loss(&a, &b), 0.0);
    }

    #[test]
    fn test_mse_difference() {
        let a = [0.0f32; 100];
        let b = [1.0f32; 100];
        assert_eq!(mse_loss(&a, &b), 1.0);
    }

    #[test]
    fn test_packet_to_ir_node_opcode() {
        let mut pkt = a2x_sigma::packet::SigmaPacket::new();
        pkt.intent.operators.push(IntentOp::Synthesis);
        let node = packet_to_ir_node(&pkt, 0);
        assert_eq!(node.opcode, Opcode::Bind);
    }

    #[test]
    fn test_intent_to_index_roundtrip() {
        let ops = [
            IntentOp::Synthesis,
            IntentOp::Split,
            IntentOp::Star,
            IntentOp::Delay,
            IntentOp::Contradiction,
            IntentOp::Lightning,
            IntentOp::Warning,
            IntentOp::Parallel,
            IntentOp::Merge,
            IntentOp::Cancel,
        ];
        let indices: Vec<usize> = ops.iter().map(|&op| intent_to_index(op)).collect();
        let unique: std::collections::HashSet<usize> = indices.iter().cloned().collect();
        assert_eq!(unique.len(), indices.len());
    }

    #[test]
    fn test_encode_with_blake3_nonzero() {
        let node = IrNode {
            id: IrNodeId(0),
            opcode: Opcode::Bind,
            operands: vec![],
            control_flow: vec![],
            metadata: IrMetadata::default(),
        };
        let omega = encode_with_blake3(&node);
        let nonzero = omega.data.iter().filter(|&&x| x != 0.0).count();
        assert!(
            nonzero > 0,
            "Blake3 encoding should produce non-zero values"
        );
    }
}
