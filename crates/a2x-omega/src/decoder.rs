// See plans/02-omega-compiler.md §5 (DecompileToSigma trait)
// Phase 3.1: real Ω → Σ∞ decoder.
//
// The Ω encoder (`compiler.rs::encode_instruction`) projects each IRNode's
// opcode into the Ω_I region via:
//
//     let hash = blake3::hash(&node.opcode.as_u8().to_le_bytes());
//     for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_I) {
//         packet.intent_slice_mut()[j] = byte as f32 / 255.0;
//     }
//
// Inverting that projection gives us a deterministic Ω → Opcode reverse
// lookup: read the first `HASH_LEN = 32` f32 slots of `intent_slice`,
// rescale × 255 → u8, and compare against the pre-baked Blake3(opcode) for
// every supported Opcode whose intent operator the compiler recognises.
// On match, recover the canonical IntentOp and emit a SigmaPacket with
// that operator.
//
// The reverse map is *partial*: only opcodes with an explicit IntentOp
// mapping (Bind/Synthesis, Differentiate/Split, Ground/Star, Evolve/Delay,
// Reflect/Contradiction, Plan/Lightning, Actuate/Warning, Fork/Parallel,
// Merge/Merge, Halt/Cancel) decode. Control-flow opcodes (Jump/Branch/Call/
// Return) and Custom(_) have no canonical Σ intent operator — they decode
// to `DecompileError::NoMatchingOperator` for now. The OmegaPacket's
// Ω_C/Ω_P/Ω_D regions still carry operand + control-flow + metadata signal
// in latent form; reconstructing those back into Σ fields is a Phase 4
// follow-on (learned decoder vs hash inverse).

use std::collections::HashMap;

use a2x_core::Opcode;
use a2x_sigma::intent::IntentOp;
use a2x_sigma::packet::SigmaPacket;

use crate::error::DecompileError;
use crate::packet::{OmegaPacket, SIZE_I};

/// Blake3 output length. The encoder uses this many bytes to seed Ω_I;
/// only that prefix of `intent_slice` carries the opcode signal.
const HASH_LEN: usize = 32;

/// Compute Blake3(Opcode::Nop) at runtime. Nop is a valid program
/// instruction with no canonical Σ intent operator — it decodes to an
/// empty `SigmaPacket` rather than `Err(NoMatchingOperator)`.
fn nop_hash() -> [u8; 32] {
    *blake3::hash(&[Opcode::Nop.as_u8()]).as_bytes()
}

/// Trait for decompiling Ω tensor packets back into Σ∞ symbolic form.
///
/// Used for debugging, logging, and inspection of compiled programs. Phase 3.1
/// resolves opcodes for the 10 IntentOp-mapped standard opcodes; everything
/// else falls through to `DecompileError::NoMatchingOperator`.
pub trait DecompileToSigma: Sized {
    type Error;

    /// Attempt to reconstruct a Σ∞ packet from an Ω tensor.
    fn decompile(packet: &OmegaPacket) -> Result<Self, Self::Error>;
}

/// Map a standard Opcode to its canonical Σ∞ IntentOp (as chosen by
/// `compiler.rs::build_ir`). Branch/Jump/Call/Return/Custom(_) have no
/// canonical mapping → returns `None` and the decoder errors out.
pub(crate) fn opcode_to_intent(op: Opcode) -> Option<IntentOp> {
    match op {
        Opcode::Bind => Some(IntentOp::Synthesis),        // ✣
        Opcode::Differentiate => Some(IntentOp::Split),   // ⩨
        Opcode::Ground => Some(IntentOp::Star),           // ✦
        Opcode::Evolve => Some(IntentOp::Delay),          // ⧖
        Opcode::Reflect => Some(IntentOp::Contradiction), // ⟁
        Opcode::Plan => Some(IntentOp::Lightning),        // ⚡
        Opcode::Actuate => Some(IntentOp::Warning),       // ⚠
        Opcode::Fork => Some(IntentOp::Parallel),         // ⩫
        Opcode::Merge => Some(IntentOp::Merge),           // ⩪
        Opcode::Halt => Some(IntentOp::Cancel),           // ✕
        Opcode::Nop
        | Opcode::Jump
        | Opcode::Branch
        | Opcode::Call
        | Opcode::Return
        | Opcode::Custom(_) => None,
    }
}

/// Reverse-lookup table: Blake3(opcode byte) → canonical IntentOp.
///
/// Built from `opcode_to_intent` to avoid duplicating the mapping.
/// Cheap (10 entries, 32-byte keys) and thread-safe (no shared state).
fn intent_hash_table() -> HashMap<[u8; 32], IntentOp> {
    let all_opcodes = [
        Opcode::Bind,
        Opcode::Differentiate,
        Opcode::Ground,
        Opcode::Evolve,
        Opcode::Reflect,
        Opcode::Plan,
        Opcode::Actuate,
        Opcode::Fork,
        Opcode::Merge,
        Opcode::Halt,
    ];
    all_opcodes
        .iter()
        .filter_map(|&op| {
            let intent = opcode_to_intent(op)?;
            let hash = *blake3::hash(&[op.as_u8()]).as_bytes();
            Some((hash, intent))
        })
        .collect()
}

/// Extract the first `HASH_LEN` bytes of the intent region.
///
/// The encoder writes `byte / 255.0` per slot, so the inverse is
/// `(f32.clamp(0,1) * 255.0).round() as u8`. Clamp guards against any
/// latent-precision drift that might produce a value just outside [0,1]
/// after a roundtrip through the encoder and a future learned encoder
/// (e.g., the Ω_C/Ω_P/Ω_D encoding mutating nearby slots).
fn extract_intent_hash(packet: &OmegaPacket) -> [u8; 32] {
    let mut bytes = [0u8; HASH_LEN];
    let slice = packet.intent_slice();
    let n = HASH_LEN.min(SIZE_I);
    for (slot, byte) in bytes.iter_mut().enumerate().take(n) {
        let f = slice[slot].clamp(0.0, 1.0);
        *byte = (f * 255.0).round() as u8;
    }
    bytes
}

impl DecompileToSigma for SigmaPacket {
    type Error = DecompileError;

    fn decompile(packet: &OmegaPacket) -> Result<Self, Self::Error> {
        let hash = extract_intent_hash(packet);

        // Nop is a valid program instruction with no intent operator —
        // decode it to an empty packet rather than erroring.
        if hash == nop_hash() {
            return Ok(SigmaPacket::new());
        }

        let table = intent_hash_table();
        match table.get(&hash) {
            Some(intent) => {
                let mut pkt = SigmaPacket::new();
                pkt.intent.operators.push(*intent);
                Ok(pkt)
            }
            None => Err(DecompileError::NoMatchingOperator),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CompileToOmega;
    use crate::packet::OmegaPacket;
    use crate::passes::OptimizationLevel;
    use a2x_sigma::parse_program;

    #[test]
    fn test_decompile_zero_packet_returns_error() {
        // All-zero Ω packet — intent slice is zeros, but no Blake3 hash of
        // any standard opcode is itself all zeros, so reverse lookup fails.
        let pkt: OmegaPacket = OmegaPacket::zeros();
        let result = SigmaPacket::decompile(&pkt);
        assert!(matches!(result, Err(DecompileError::NoMatchingOperator)));
    }

    #[test]
    fn test_extract_intent_hash_byte_round_trip() {
        // Exercise the projection inverse: write a known byte pattern into
        // a packet, read it back through `extract_intent_hash` — bytes must
        // match exactly (within f32 quantization tolerance of 1/255).
        let mut pkt: OmegaPacket = OmegaPacket::zeros();
        for (i, slot) in pkt.intent_slice_mut().iter_mut().enumerate().take(HASH_LEN) {
            let byte = (i as u8).wrapping_mul(13);
            *slot = byte as f32 / 255.0;
        }
        let hash = extract_intent_hash(&pkt);
        for (i, &byte) in hash.iter().enumerate().take(HASH_LEN) {
            let original = (i as u8).wrapping_mul(13);
            assert_eq!(
                byte, original,
                "byte at slot {i} round-tripped: expected 0x{:02x}, got 0x{:02x}",
                original, byte
            );
        }
    }

    #[test]
    fn test_opcode_to_intent_total_count() {
        // Sanity: exactly 10 opcodes have IntentOp mappings; the rest are
        // control flow + Custom + Nop which all return None.
        let mapped = [
            Opcode::Bind,
            Opcode::Differentiate,
            Opcode::Ground,
            Opcode::Evolve,
            Opcode::Reflect,
            Opcode::Plan,
            Opcode::Actuate,
            Opcode::Fork,
            Opcode::Merge,
            Opcode::Halt,
        ];
        for op in mapped {
            assert!(
                opcode_to_intent(op).is_some(),
                "{:?} should map to an IntentOp",
                op
            );
        }
        for op in [
            Opcode::Nop,
            Opcode::Jump,
            Opcode::Branch,
            Opcode::Call,
            Opcode::Return,
            Opcode::Custom(0xAA),
        ] {
            assert!(
                opcode_to_intent(op).is_none(),
                "{:?} should NOT map to an IntentOp",
                op
            );
        }
    }

    #[test]
    fn test_intent_hash_table_size() {
        let table = intent_hash_table();
        assert_eq!(table.len(), 10, "expected 10 opcode → intent entries");
    }

    #[test]
    fn test_intent_hash_table_blake3_consistency() {
        // The hash table must agree with what the encoder emits. For each
        // mapped opcode, build the canonical hash and confirm the table's
        // entry matches what encode_instruction would write.
        let table = intent_hash_table();
        for op in [
            Opcode::Bind,
            Opcode::Differentiate,
            Opcode::Ground,
            Opcode::Evolve,
            Opcode::Reflect,
            Opcode::Plan,
            Opcode::Actuate,
            Opcode::Fork,
            Opcode::Merge,
            Opcode::Halt,
        ] {
            let expected_hash = *blake3::hash(&[op.as_u8()]).as_bytes();
            let intent = opcode_to_intent(op).expect("mapped opcode");
            assert_eq!(
                table.get(&expected_hash),
                Some(&intent),
                "{:?} → {:?} not in table",
                op,
                intent
            );
        }
    }

    #[test]
    fn test_decompile_handcrafted_plan_packet() {
        // Manually construct an Ω_I region that exactly mirrors what the
        // encoder would produce for Opcode::Plan (0x6). Decoding should
        // recover Lightning — independent of the compiler pipeline.
        let mut pkt: OmegaPacket = OmegaPacket::zeros();
        let hash = *blake3::hash(&[Opcode::Plan.as_u8()]).as_bytes();
        for (j, &byte) in hash.iter().enumerate().take(HASH_LEN) {
            pkt.intent_slice_mut()[j] = byte as f32 / 255.0;
        }
        let sigma = SigmaPacket::decompile(&pkt).expect("Plan must decode");
        assert_eq!(sigma.intent.operators, vec![IntentOp::Lightning]);
        // Other Σ fields are intentionally empty in Phase 3.1 —
        // operand/context/plan recovery needs a richer encoding protocol.
        assert!(sigma.context.is_empty());
        assert!(sigma.plan.is_empty());
        assert!(sigma.data.is_empty());
    }

    #[test]
    fn test_decompile_handcrafted_all_mapped_opcodes() {
        for op in [
            Opcode::Bind,
            Opcode::Differentiate,
            Opcode::Ground,
            Opcode::Evolve,
            Opcode::Reflect,
            Opcode::Plan,
            Opcode::Actuate,
            Opcode::Fork,
            Opcode::Merge,
            Opcode::Halt,
        ] {
            let mut pkt: OmegaPacket = OmegaPacket::zeros();
            let hash = *blake3::hash(&[op.as_u8()]).as_bytes();
            for (j, &byte) in hash.iter().enumerate().take(HASH_LEN) {
                pkt.intent_slice_mut()[j] = byte as f32 / 255.0;
            }
            let sigma = SigmaPacket::decompile(&pkt).unwrap_or_else(|e| {
                panic!("{op:?} should decode, got {e:?}");
            });
            let expected = opcode_to_intent(op).unwrap();
            assert_eq!(sigma.intent.operators, vec![expected], "{op:?}");
        }
    }

    #[test]
    fn test_decompile_handcrafted_nop_returns_empty_packet() {
        // Opcode::Nop is a valid program instruction — it should decode
        // to an empty SigmaPacket, not an error.
        let mut pkt: OmegaPacket = OmegaPacket::zeros();
        let hash = nop_hash();
        for (j, byte) in hash.iter().enumerate().take(HASH_LEN) {
            pkt.intent_slice_mut()[j] = *byte as f32 / 255.0;
        }
        let sigma = SigmaPacket::decompile(&pkt).expect("Nop must decode");
        assert!(sigma.intent.operators.is_empty(), "Nop → empty intent");
        assert!(sigma.context.is_empty());
        assert!(sigma.plan.is_empty());
        assert!(sigma.data.is_empty());
    }

    #[test]
    fn test_decompile_handcrafted_branch_returns_error() {
        // Opcode::Branch has no canonical IntentOp — even if we feed a
        // hand-crafted Ω_I region with the Branch hash, decode must error.
        let mut pkt: OmegaPacket = OmegaPacket::zeros();
        let hash = *blake3::hash(&[Opcode::Branch.as_u8()]).as_bytes();
        for (j, &byte) in hash.iter().enumerate().take(HASH_LEN) {
            pkt.intent_slice_mut()[j] = byte as f32 / 255.0;
        }
        let result = SigmaPacket::decompile(&pkt);
        assert!(matches!(result, Err(DecompileError::NoMatchingOperator)));
    }

    #[test]
    fn test_compile_then_decompile_roundtrip_preserves_intent_operator() {
        // End-to-end: parse a Σ∞ program, compile to Ω, then decompile the
        // first packet. Decoded intent operator must match the source.
        let input = "⟦Σ∞⟧⟬I:✣ ∷ C:⟨a⟩ ∷ P:⥂ ∷ D:⌬⟭";
        let prog = parse_program(input).unwrap();
        let omega = prog.compile(OptimizationLevel::Light).expect("compile");
        assert!(!omega.instructions.is_empty());

        let sigma =
            SigmaPacket::decompile(&omega.instructions[0]).expect("encoded packet should decode");
        // Source: ✣ → Synthesis. Compiler maps Synthesis → Opcode::Bind.
        // Decoder inverts Opcode::Bind → Synthesis. Round-trip preserves intent.
        assert_eq!(sigma.intent.operators, vec![IntentOp::Synthesis]);
    }

    #[test]
    fn test_compile_then_decompile_multi_packet_first_intent_matches() {
        // A two-packet Σ∞ program with distinct intent operators on each
        // packet; decompile of each Ω packet must recover the matching
        // intent operator (independent of position in stream).
        let input = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨a⟩ ∷ P:⥂ ∷ D:⌵⟭⟦Σ∞⟧⟬I:✕ ∷ C:⟘ ∷ P:⤉ ∷ D:⟘⟭";
        let prog = parse_program(input).unwrap();
        let omega = prog.compile(OptimizationLevel::Light).expect("compile");
        assert_eq!(omega.instructions.len(), 2, "two source packets");

        let p0 = SigmaPacket::decompile(&omega.instructions[0]).expect("first packet must decode");
        let p1 = SigmaPacket::decompile(&omega.instructions[1]).expect("second packet must decode");
        // Source: ✦ → Ground → Opcode::Ground → reverse → Star.
        assert_eq!(p0.intent.operators, vec![IntentOp::Star]);
        // Source: ✕ → Halt → Opcode::Halt → reverse → Cancel.
        assert_eq!(p1.intent.operators, vec![IntentOp::Cancel]);
    }
}
