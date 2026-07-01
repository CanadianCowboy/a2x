// See plans/02-omega-compiler.md §6
//
// Encoder: projects IR nodes into Ω tensor packets.
//
// Phase 0 uses deterministic Blake3-based projection of opcode, operands,
// control flow, and metadata into the 4 tensor regions (I, C, P, D).
// Future phases will use learned neural encoders (see learned_encoder.rs).

use crate::ir::IrNode;
use crate::packet::{OmegaPacket, SIZE_C, SIZE_D, SIZE_I, SIZE_P};

/// Encode a single IR node as an Ω tensor packet.
///
/// Phase 0: deterministic Blake3-based projection of the opcode, operands,
/// and control flow into the 4 tensor regions.
pub fn encode_instruction(node: &IrNode) -> OmegaPacket<29796> {
    let mut packet = OmegaPacket::<29796>::zeros();

    // Project opcode → intent region (I)
    let hash = blake3::hash(&node.opcode.as_u8().to_le_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_I) {
        packet.intent_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project operands → context region (C)
    let op_str = format!("{:?}", &node.operands);
    let hash = blake3::hash(op_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_C) {
        packet.context_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project control flow → plan region (P)
    let cf_str = format!("{:?}", &node.control_flow);
    let hash = blake3::hash(cf_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_P) {
        packet.plan_slice_mut()[j] = byte as f32 / 255.0;
    }

    // Project metadata → data region (D)
    let meta_str = format!("{:?}", &node.metadata.source_index);
    let hash = blake3::hash(meta_str.as_bytes());
    for (j, &byte) in hash.as_bytes().iter().enumerate().take(SIZE_D) {
        packet.data_slice_mut()[j] = byte as f32 / 255.0;
    }

    packet
}
