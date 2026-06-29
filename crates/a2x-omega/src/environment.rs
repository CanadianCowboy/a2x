// Phase 4.4 — Simulated Environment: random valid Σ∞ program generator.
//
// Generates synthetic training data for the learned encoder/decoder.
// Each generated program is a random sequence of valid Σ∞ packets with
// randomly selected intent operators, context labels, plan operators,
// and data payloads.

use a2x_sigma::intent::IntentOp;
use a2x_sigma::packet::SigmaPacket;
use a2x_sigma::plan::PlanOp;
use a2x_sigma::program::SigmaProgram;

/// Simple deterministic PRNG for reproducible training data.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() as f32) / (u64::MAX as f32)
    }

    fn range(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }
}

/// Pool of intent operators for random selection.
const INTENT_OPS: &[IntentOp] = &[
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

/// Pool of plan operators for random selection.
const PLAN_OPS: &[PlanOp] = &[
    PlanOp::Sequential,
    PlanOp::Branch,
    PlanOp::Swarm,
    PlanOp::Merge,
    PlanOp::Descend,
    PlanOp::Ascend,
];

/// Pool of context labels for random selection.
const LABELS: &[&str] = &[
    "sys", "src", "out", "log", "net", "fs", "mem", "cpu", "data", "ctx", "env", "io", "db",
    "cache", "proc", "sig",
];

/// Pool of data payload sizes (in f32 values).
const PAYLOAD_SIZES: &[usize] = &[0, 1, 2, 4, 8];

/// Generate a single random Σ∞ packet.
fn generate_packet(rng: &mut SimpleRng) -> SigmaPacket {
    let mut p = SigmaPacket::new();

    // Random intent operator
    let intent_idx = rng.range(INTENT_OPS.len());
    p.intent.operators.push(INTENT_OPS[intent_idx]);

    // Random context labels (0..=3 labels)
    let num_labels = rng.range(4);
    for _ in 0..num_labels {
        let label_idx = rng.range(LABELS.len());
        p.context.labels.push(LABELS[label_idx].to_string());
    }

    // Random plan operator
    let plan_idx = rng.range(PLAN_OPS.len());
    p.plan.operators.push(PLAN_OPS[plan_idx]);

    // Random data payload
    let payload_size = PAYLOAD_SIZES[rng.range(PAYLOAD_SIZES.len())];
    let mut payload = Vec::with_capacity(payload_size * 4);
    for _ in 0..payload_size {
        let v = rng.next_f32() * 2.0 - 1.0; // [-1, 1]
        payload.extend_from_slice(&v.to_le_bytes());
    }
    p.data.payload = payload;

    p
}

/// Generate a random Σ∞ program with `num_packets` instructions.
pub fn generate_program(seed: u64, num_packets: usize) -> SigmaProgram {
    let mut rng = SimpleRng::new(seed);
    let mut prog = SigmaProgram::new();
    for _ in 0..num_packets {
        let packet = generate_packet(&mut rng);
        prog.push(packet);
    }
    prog
}

/// Generate a batch of random programs with varying sizes.
pub fn generate_batch(
    seed: u64,
    batch_size: usize,
    min_packets: usize,
    max_packets: usize,
) -> Vec<SigmaProgram> {
    let mut rng = SimpleRng::new(seed);
    let mut programs = Vec::with_capacity(batch_size);
    let safe_min = min_packets.min(max_packets);
    let range = max_packets.saturating_sub(min_packets);
    for i in 0..batch_size {
        let num_packets = safe_min + rng.range(range.max(1));
        let program = generate_program(
            seed.wrapping_add((i as u64).wrapping_mul(1000)),
            num_packets,
        );
        programs.push(program);
    }
    programs
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_program_nonempty() {
        let prog = generate_program(42, 5);
        assert_eq!(prog.instructions.len(), 5);
    }

    #[test]
    fn test_generate_program_deterministic() {
        let p1 = generate_program(42, 3);
        let p2 = generate_program(42, 3);
        assert_eq!(p1.instructions.len(), p2.instructions.len());
        // Same seed → same packets
        for (a, b) in p1.instructions.iter().zip(p2.instructions.iter()) {
            assert_eq!(format!("{}", a), format!("{}", b));
        }
    }

    #[test]
    fn test_generate_program_different_seeds() {
        let p1 = generate_program(1, 3);
        let p2 = generate_program(2, 3);
        // Different seeds should generally produce different programs
        let same = p1
            .instructions
            .iter()
            .zip(p2.instructions.iter())
            .filter(|(a, b)| format!("{}", a) == format!("{}", b))
            .count();
        // Unlikely all 3 packets are identical with different seeds
        assert!(same < 3);
    }

    #[test]
    fn test_generate_batch_size() {
        let batch = generate_batch(0, 10, 1, 5);
        assert_eq!(batch.len(), 10);
        for prog in &batch {
            assert!(!prog.is_empty());
        }
    }

    #[test]
    fn test_generate_packet_has_intent() {
        let mut rng = SimpleRng::new(42);
        let packet = generate_packet(&mut rng);
        assert!(!packet.intent.operators.is_empty());
    }
}
