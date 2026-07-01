// See plans/01-sigma-language.md §24 (Binary ISA Encoding)
//
// Compact binary instruction format for Σ∞ packets.
//
// Layout (per instruction):
//   ┌──────────┬──────────┬──────────┬──────────┬──────────┐
//   │  Header  │  Operand │  Control │   Data   │ Checksum │
//   │  1 byte  │ 4 bytes  │ 2 bytes  │ variable │  4 bytes │
//   └──────────┴──────────┴──────────┴──────────┴──────────┘
//
//   Header:  Protocol(2 bits) | Opcode(4 bits) | Flags(2 bits)
//   Operand: Mode(2 bits) | Target(30 bits)
//   Control: FlowOp(4 bits) | Target(12 bits)
//   Data:    Length(2 bytes BE) | Payload (variable, up to 64 KiB)
//   Checksum: CRC32 of Header+Operand+Control+Data (4 bytes BE)
//
// Minimum size: 13 bytes per instruction (header+operand+control+data_len+crc32).

use crate::context::ContextOp;
use crate::data::DataOp;
use crate::intent::IntentOp;
use crate::packet::{ContextField, DataField, IntentField, PlanField, SigmaPacket};
use crate::plan::PlanOp;
use a2x_core::ProtocolId;

// ── Header constants ──────────────────────────────────────────────────────

/// Protocol field bits.
const PROTOCOL_SIGMA: u8 = 0b00;
const PROTOCOL_OMEGA: u8 = 0b01;
// const PROTOCOL_RESERVED: u8 = 0b10;
const PROTOCOL_RAW: u8 = 0b11;

/// Execution mode flags (2 bits in header).
const FLAG_NORMAL: u8 = 0b00;
const FLAG_IMMEDIATE: u8 = 0b01; // ⚡ Lightning
const FLAG_EXPLORE: u8 = 0b10; // ✦ Star
const FLAG_SAFE: u8 = 0b11; // ⚠ Warning

/// CCS VM opcodes (4 bits in header).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOpcode {
    Nop = 0x0,
    Bind = 0x1,
    Diff = 0x2,
    Grnd = 0x3,
    Evol = 0x4,
    Refl = 0x5,
    Plan = 0x6,
    Act = 0x7,
    Jmp = 0x8,
    Br = 0x9,
    Call = 0xA,
    Ret = 0xB,
    Fork = 0xC,
    Merge = 0xD,
    Halt = 0xE,
    Custom = 0xF,
}

impl BinaryOpcode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v & 0x0F {
            0x0 => Some(BinaryOpcode::Nop),
            0x1 => Some(BinaryOpcode::Bind),
            0x2 => Some(BinaryOpcode::Diff),
            0x3 => Some(BinaryOpcode::Grnd),
            0x4 => Some(BinaryOpcode::Evol),
            0x5 => Some(BinaryOpcode::Refl),
            0x6 => Some(BinaryOpcode::Plan),
            0x7 => Some(BinaryOpcode::Act),
            0x8 => Some(BinaryOpcode::Jmp),
            0x9 => Some(BinaryOpcode::Br),
            0xA => Some(BinaryOpcode::Call),
            0xB => Some(BinaryOpcode::Ret),
            0xC => Some(BinaryOpcode::Fork),
            0xD => Some(BinaryOpcode::Merge),
            0xE => Some(BinaryOpcode::Halt),
            0xF => Some(BinaryOpcode::Custom),
            _ => None,
        }
    }
}

/// Operand addressing mode (2 bits).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressingMode {
    /// Label index (into program's label table, up to 1 Gi labels).
    Label = 0b00,
    /// Direct NodeId (numeric world-graph node reference).
    NodeId = 0b01,
    /// StateField region index.
    Region = 0b10,
    /// Reserved.
    Reserved = 0b11,
}

/// Control flow operation (4 bits in control field).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowOp {
    Sequential = 0x0,  // IP += 1
    JumpAbs = 0x1,     // Jump to absolute address
    JumpRel = 0x2,     // Jump by signed offset
    BranchTrue = 0x3,  // Branch if true
    BranchFalse = 0x4, // Branch if false
    Call = 0x5,        // Push return addr, jump
    Return = 0x6,      // Pop return addr
    Fork = 0x7,        // Parallel sub-programs
    MergeWait = 0x8,   // Wait for all forks
    Halt = 0x9,        // Stop execution
}

// ── Error types ───────────────────────────────────────────────────────────

/// Error during binary encoding or decoding of an instruction.
#[derive(Clone, Debug, PartialEq)]
pub enum BinaryError {
    /// Input buffer too short.
    TooShort { need: usize, have: usize },
    /// Invalid opcode value.
    InvalidOpcode(u8),
    /// Invalid protocol bits.
    InvalidProtocol(u8),
    /// Invalid addressing mode.
    InvalidAddressingMode(u8),
    /// Invalid flow op.
    InvalidFlowOp(u8),
    /// CRC32 checksum mismatch.
    ChecksumMismatch { computed: u32, expected: u32 },
    /// No intent operator to map to an opcode (empty packet).
    EmptyPacket,
    /// Unknown operator can't be encoded.
    Unencodable(&'static str),
}

impl std::fmt::Display for BinaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryError::TooShort { need, have } => {
                write!(f, "buffer too short: need {} bytes, have {}", need, have)
            }
            BinaryError::InvalidOpcode(v) => write!(f, "invalid opcode: 0x{:02X}", v),
            BinaryError::InvalidProtocol(v) => write!(f, "invalid protocol bits: 0b{:02b}", v),
            BinaryError::InvalidAddressingMode(v) => {
                write!(f, "invalid addressing mode: 0b{:02b}", v)
            }
            BinaryError::InvalidFlowOp(v) => write!(f, "invalid flow op: 0x{:X}", v),
            BinaryError::ChecksumMismatch { computed, expected } => {
                write!(
                    f,
                    "checksum mismatch: computed 0x{:08X}, expected 0x{:08X}",
                    computed, expected
                )
            }
            BinaryError::EmptyPacket => write!(f, "cannot encode empty packet"),
            BinaryError::Unencodable(s) => write!(f, "unencodable operator: {}", s),
        }
    }
}

impl std::error::Error for BinaryError {}

// ── CRC32 ─────────────────────────────────────────────────────────────────

/// CRC32 checksum using the standard IEEE 802.3 polynomial.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

// ── Encoding ──────────────────────────────────────────────────────────────

/// Encode a single Σ∞ packet into a binary instruction buffer.
///
/// Returns the encoded byte vector including the 4-byte CRC32 checksum.
pub fn encode_instruction(packet: &SigmaPacket) -> Result<Vec<u8>, BinaryError> {
    // Determine the primary opcode from intent operators.
    let (opcode, flags) = map_to_opcode_and_flags(&packet.intent, &packet.plan);

    // Protocol
    let protocol_bits = match packet.protocol {
        ProtocolId::Sigma => PROTOCOL_SIGMA,
        ProtocolId::Omega => PROTOCOL_OMEGA,
        ProtocolId::Raw => PROTOCOL_RAW,
    };

    // Build header: Protocol(2) | Opcode(4) | Flags(2)
    let header: u8 = (protocol_bits << 6) | ((opcode as u8) << 2) | (flags & 0b11);

    // Build operand (4 bytes): Mode(2) | Target(30)
    let operand: u32 = encode_operand(&packet.context);

    // Build control (2 bytes): FlowOp(4) | Target(12)
    let control: u16 = encode_control(&packet.plan);

    // Build data payload.
    let data = encode_data(&packet.data);

    // Frame without checksum.
    let mut frame = Vec::with_capacity(11 + data.len());
    frame.push(header);
    frame.extend_from_slice(&operand.to_be_bytes());
    frame.extend_from_slice(&control.to_be_bytes());
    if data.len() > u16::MAX as usize {
        return Err(BinaryError::Unencodable("data payload exceeds 64 KiB"));
    }
    frame.extend_from_slice(&(data.len() as u16).to_be_bytes());
    frame.extend_from_slice(&data);

    // CRC32 of header+operand+control+data.
    let checksum = crc32(&frame);
    frame.extend_from_slice(&checksum.to_be_bytes());

    Ok(frame)
}

/// Map intent and plan operators to a binary opcode and execution flags.
fn map_to_opcode_and_flags(intent: &IntentField, plan: &PlanField) -> (BinaryOpcode, u8) {
    // Determine flags from execution-mode intents.
    let flags = {
        let mut f = FLAG_NORMAL;
        for op in &intent.operators {
            match op {
                IntentOp::Lightning => f = FLAG_IMMEDIATE,
                IntentOp::Warning => f = FLAG_SAFE,
                IntentOp::Star => f = FLAG_EXPLORE,
                _ => {}
            }
        }
        f
    };

    // Determine primary opcode: intent operators first, then plan operators.
    for op in &intent.operators {
        match op {
            IntentOp::Synthesis => return (BinaryOpcode::Bind, flags),
            IntentOp::Split => return (BinaryOpcode::Diff, flags),
            IntentOp::Star => return (BinaryOpcode::Grnd, flags),
            IntentOp::Cancel => return (BinaryOpcode::Halt, flags),
            _ => {}
        }
    }

    // Check context-derived ops in the intent (indirect).
    for op in &intent.operators {
        match op {
            IntentOp::Accelerate => return (BinaryOpcode::Act, flags),
            IntentOp::Delay => return (BinaryOpcode::Nop, flags),
            IntentOp::Parallel => return (BinaryOpcode::Fork, flags),
            IntentOp::Merge => return (BinaryOpcode::Merge, flags),
            IntentOp::Contradiction => return (BinaryOpcode::Halt, flags),
            _ => {}
        }
    }

    // Fall back to plan operators.
    for op in &plan.operators {
        match op {
            PlanOp::Branch => return (BinaryOpcode::Br, flags),
            PlanOp::Descend => return (BinaryOpcode::Call, flags),
            PlanOp::Ascend => return (BinaryOpcode::Ret, flags),
            PlanOp::Swarm => return (BinaryOpcode::Fork, flags),
            PlanOp::Merge => return (BinaryOpcode::Merge, flags),
            PlanOp::Escalate => return (BinaryOpcode::Act, flags),
            _ => {}
        }
    }

    // Default: NOP
    (BinaryOpcode::Nop, flags)
}

/// Encode the context field into a 4-byte operand value.
fn encode_operand(context: &ContextField) -> u32 {
    // Use first label as the target (label index), or 0 if none.
    // For a full implementation, labels would map to indices in the program's
    // label table. Here we encode the label as a simple hash.
    if let Some(label) = context.labels.first() {
        let mode = AddressingMode::Label as u32;
        // Simple hash of the label into 30 bits.
        let target = simple_hash32(label) & 0x3FFFFFFF;
        (mode << 30) | target
    } else if !context.operators.is_empty() {
        // Context operator without label — encode first operator as region index.
        let mode = AddressingMode::Region as u32;
        let region_idx = context.operators[0] as u32 & 0x3FFFFFFF;
        (mode << 30) | region_idx
    } else {
        0 // No operand.
    }
}

/// Encode the plan field into a 2-byte control value.
fn encode_control(plan: &PlanField) -> u16 {
    let flow = if plan.operators.is_empty() {
        FlowOp::Sequential
    } else {
        match plan.operators[0] {
            PlanOp::Sequential => FlowOp::Sequential,
            PlanOp::Branch => FlowOp::BranchTrue,
            PlanOp::Descend => FlowOp::Call,
            PlanOp::Ascend => FlowOp::Return,
            PlanOp::Swarm => FlowOp::Fork,
            PlanOp::Merge => FlowOp::MergeWait,
            PlanOp::Escalate => FlowOp::Halt,
            PlanOp::Recursive => FlowOp::Call, // Recursion = call self
            _ => FlowOp::Sequential,
        }
    };

    (flow as u16) << 12 // target = 0 for sequential
}

/// Encode the data field into a byte payload.
fn encode_data(data: &DataField) -> Vec<u8> {
    if data.payload.is_empty() && data.operators.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();

    // Encode data operators as a compact list (1 byte per operator).
    if !data.operators.is_empty() {
        out.push(data.operators.len() as u8);
        for op in &data.operators {
            out.push(*op as u8);
        }
    }

    // Append raw payload.
    out.extend_from_slice(&data.payload);

    out
}

/// Simple 32-bit hash for label encoding (FNV-1a inspired).
fn simple_hash32(s: &str) -> u32 {
    let mut hash: u32 = 0x811C_9DC5;
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

// ── Decoding ──────────────────────────────────────────────────────────────

/// Decode a single binary instruction into a Σ∞ packet.
///
/// Returns the decoded packet and the number of bytes consumed.
pub fn decode_instruction(buf: &[u8]) -> Result<(SigmaPacket, usize), BinaryError> {
    // Minimum frame: header(1) + operand(4) + control(2) + data_len(2) + crc32(4) = 13
    if buf.len() < 13 {
        return Err(BinaryError::TooShort {
            need: 13,
            have: buf.len(),
        });
    }

    let header = buf[0];
    let protocol_bits = (header >> 6) & 0b11;
    let opcode_bits = (header >> 2) & 0b1111;
    let flags = header & 0b11;

    let operand = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]);
    let control = u16::from_be_bytes([buf[5], buf[6]]);
    let data_len = u16::from_be_bytes([buf[7], buf[8]]) as usize;

    if buf.len() < 13 + data_len {
        return Err(BinaryError::TooShort {
            need: 13 + data_len,
            have: buf.len(),
        });
    }

    let data = &buf[9..9 + data_len];
    let checksum_offset = 9 + data_len;

    let expected_checksum = u32::from_be_bytes([
        buf[checksum_offset],
        buf[checksum_offset + 1],
        buf[checksum_offset + 2],
        buf[checksum_offset + 3],
    ]);

    // Verify CRC32.
    let computed = crc32(&buf[..checksum_offset]);
    if computed != expected_checksum {
        return Err(BinaryError::ChecksumMismatch {
            computed,
            expected: expected_checksum,
        });
    }

    let protocol = match protocol_bits {
        PROTOCOL_SIGMA => ProtocolId::Sigma,
        PROTOCOL_OMEGA => ProtocolId::Omega,
        PROTOCOL_RAW => ProtocolId::Raw,
        _ => return Err(BinaryError::InvalidProtocol(protocol_bits)),
    };

    let opcode =
        BinaryOpcode::from_u8(opcode_bits).ok_or(BinaryError::InvalidOpcode(opcode_bits))?;

    let intent = decode_intent(opcode, flags);
    let context = decode_context(operand);
    let plan = decode_plan(control);
    let data_field = decode_data(data);

    let consumed = 13 + data_len;
    Ok((
        SigmaPacket {
            protocol,
            intent,
            context,
            plan,
            data: data_field,
        },
        consumed,
    ))
}

/// Decode opcode + flags into an intent field.
fn decode_intent(opcode: BinaryOpcode, flags: u8) -> IntentField {
    let mut ops = Vec::new();

    // Execution mode flags.
    match flags {
        FLAG_IMMEDIATE => ops.push(IntentOp::Lightning),
        FLAG_SAFE => ops.push(IntentOp::Warning),
        FLAG_EXPLORE => ops.push(IntentOp::Star),
        _ => {}
    }

    // Primary opcode mapping.
    match opcode {
        BinaryOpcode::Nop => {}
        BinaryOpcode::Bind => ops.push(IntentOp::Synthesis),
        BinaryOpcode::Diff => ops.push(IntentOp::Split),
        BinaryOpcode::Grnd => {
            if flags != FLAG_EXPLORE {
                ops.push(IntentOp::Star);
            }
        }
        BinaryOpcode::Evol => {}
        BinaryOpcode::Refl => {}
        BinaryOpcode::Plan => {}
        BinaryOpcode::Act => ops.push(IntentOp::Accelerate),
        BinaryOpcode::Jmp => {}
        BinaryOpcode::Br => {}
        BinaryOpcode::Call => {}
        BinaryOpcode::Ret => {}
        BinaryOpcode::Fork => ops.push(IntentOp::Parallel),
        BinaryOpcode::Merge => ops.push(IntentOp::Merge),
        BinaryOpcode::Halt => ops.push(IntentOp::Cancel),
        BinaryOpcode::Custom => {}
    }

    IntentField { operators: ops }
}

/// Decode a 4-byte operand into a context field.
fn decode_context(operand: u32) -> ContextField {
    let mode = (operand >> 30) & 0b11;
    let target = operand & 0x3FFFFFFF;

    let mut ops = Vec::new();
    let mut labels = Vec::new();

    match mode {
        0b00 => {
            // Label mode — target is a hash, can't reverse to original label.
            // Store as numeric label reference.
            labels.push(format!("#{}", target));
        }
        0b10 => {
            // Region mode — target is a region index.
            // Map back to a context operator if possible.
            if let Some(op) = context_op_from_index(target as usize) {
                ops.push(op);
            }
        }
        _ => {
            // NodeId or Reserved — no label/operator info recoverable.
        }
    }

    ContextField {
        operators: ops,
        labels,
    }
}

/// Map a numeric index back to a ContextOp (best-effort).
fn context_op_from_index(idx: usize) -> Option<ContextOp> {
    match idx {
        0 => Some(ContextOp::Null),
        1 => Some(ContextOp::Universal),
        2 => Some(ContextOp::Compression),
        3 => Some(ContextOp::Uncertainty),
        4 => Some(ContextOp::CausalChain),
        5 => Some(ContextOp::SpatialChain),
        6 => Some(ContextOp::TemporalChain),
        7 => Some(ContextOp::Probabilistic),
        8 => Some(ContextOp::Conflict),
        9 => Some(ContextOp::Resolved),
        _ => None,
    }
}

/// Decode the control field into a plan field.
fn decode_plan(control: u16) -> PlanField {
    let flow = (control >> 12) & 0x0F;
    let _target = control & 0x0FFF;

    let mut ops = Vec::new();
    match flow {
        0x0 => ops.push(PlanOp::Sequential),
        0x1..=0x4 => ops.push(PlanOp::Branch), // JumpAbs, JumpRel, BranchTrue, BranchFalse → branch
        0x5 => ops.push(PlanOp::Descend),      // Call
        0x6 => ops.push(PlanOp::Ascend),       // Return
        0x7 => ops.push(PlanOp::Swarm),        // Fork
        0x8 => ops.push(PlanOp::Merge),        // Merge
        0x9 => ops.push(PlanOp::Escalate),     // Halt → escalate
        _ => ops.push(PlanOp::Sequential),
    }

    PlanField { operators: ops }
}

/// Decode the data payload into a data field.
fn decode_data(data: &[u8]) -> DataField {
    if data.is_empty() {
        return DataField::new();
    }

    let mut ops = Vec::new();
    let mut payload = Vec::new();
    let mut pos = 0;

    // If the first byte is a count (≤ 11), treat it as a data operator list.
    if pos < data.len() {
        let count = data[pos] as usize;
        if count > 0 && count <= 11 && pos + 1 + count <= data.len() {
            pos += 1;
            for _ in 0..count {
                let op_idx = data[pos];
                if let Some(op) = data_op_from_index(op_idx) {
                    ops.push(op);
                }
                pos += 1;
            }
        }
    }

    // Remaining bytes are raw payload.
    if pos < data.len() {
        payload = data[pos..].to_vec();
    }

    DataField {
        operators: ops,
        payload,
    }
}

fn data_op_from_index(idx: u8) -> Option<DataOp> {
    match idx {
        0 => Some(DataOp::RawTensor),
        1 => Some(DataOp::LatentVector),
        2 => Some(DataOp::GraphDelta),
        3 => Some(DataOp::DiffPatch),
        4 => Some(DataOp::Binary),
        5 => Some(DataOp::Fusion),
        6 => Some(DataOp::Streaming),
        7 => Some(DataOp::Summary),
        8 => Some(DataOp::Anomaly),
        9 => Some(DataOp::Schema),
        10 => Some(DataOp::SelfDescribing),
        _ => None,
    }
}

// ── Higher-level encode/decode ────────────────────────────────────────────

/// Encode a SigmaPacket to bytes using the binary ISA format.
pub fn to_bytes(packet: &SigmaPacket) -> Result<Vec<u8>, BinaryError> {
    encode_instruction(packet)
}

/// Decode bytes into a SigmaPacket using the binary ISA format.
///
/// Returns the decoded packet and the number of bytes consumed.
pub fn from_bytes(buf: &[u8]) -> Result<(SigmaPacket, usize), BinaryError> {
    decode_instruction(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CRC32 tests ──────────────────────────────────────────────────

    #[test]
    fn test_crc32_known_value() {
        // "123456789" → 0xCBF43926 (standard IEEE CRC32 test vector)
        let crc = crc32(b"123456789");
        assert_eq!(crc, 0xCBF43926);
    }

    // ── Encode/decode roundtrip ──────────────────────────────────────

    #[test]
    fn test_encode_decode_roundtrip_empty() {
        let packet = SigmaPacket::new();
        let encoded = encode_instruction(&packet).unwrap();
        // Minimum frame: header(1)+operand(4)+control(2)+data_len(2)+crc32(4)=13
        assert_eq!(encoded.len(), 13);
        let (decoded, consumed) = decode_instruction(&encoded).unwrap();
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.protocol, ProtocolId::Sigma);
    }

    #[test]
    fn test_encode_decode_synthesis() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Synthesis);
        let encoded = encode_instruction(&packet).unwrap();
        let (decoded, _) = decode_instruction(&encoded).unwrap();
        assert!(decoded.intent.operators.contains(&IntentOp::Synthesis));
    }

    #[test]
    fn test_encode_decode_with_flags() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Synthesis);
        packet.intent.operators.push(IntentOp::Lightning);
        let encoded = encode_instruction(&packet).unwrap();
        let (decoded, _) = decode_instruction(&encoded).unwrap();
        assert!(decoded.intent.operators.contains(&IntentOp::Synthesis));
        assert!(decoded.intent.operators.contains(&IntentOp::Lightning));
    }

    #[test]
    fn test_encode_decode_with_context() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Star);
        packet.context.operators.push(ContextOp::Compression);
        packet.context.labels.push("sys".to_string());
        let encoded = encode_instruction(&packet).unwrap();
        let (decoded, _) = decode_instruction(&encoded).unwrap();
        assert!(decoded.intent.operators.contains(&IntentOp::Star));
        // Label should survive (but may be hashed).
        assert!(!decoded.context.labels.is_empty());
    }

    #[test]
    fn test_encode_decode_with_plan() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Synthesis);
        packet.plan.operators.push(PlanOp::Swarm);
        packet.plan.operators.push(PlanOp::Escalate);
        let encoded = encode_instruction(&packet).unwrap();
        let (decoded, _) = decode_instruction(&encoded).unwrap();
        assert!(decoded.intent.operators.contains(&IntentOp::Synthesis));
        assert!(decoded.plan.operators.contains(&PlanOp::Swarm));
    }

    #[test]
    fn test_encode_decode_with_data() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Synthesis);
        packet.data.operators.push(DataOp::Binary);
        packet.data.payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let encoded = encode_instruction(&packet).unwrap();
        assert!(encoded.len() > 13); // Has data payload.
        let (decoded, _) = decode_instruction(&encoded).unwrap();
        assert!(decoded.data.operators.contains(&DataOp::Binary));
        assert_eq!(decoded.data.payload, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_decode_too_short() {
        let buf = [0u8; 4];
        let result = decode_instruction(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_checksum_error() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Synthesis);
        let mut encoded = encode_instruction(&packet).unwrap();
        // Corrupt a byte after the checksum was computed.
        encoded[1] ^= 0xFF;
        let result = decode_instruction(&encoded);
        assert!(matches!(result, Err(BinaryError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_encode_plan_operators() {
        let cases = vec![
            (PlanOp::Branch, BinaryOpcode::Br),
            (PlanOp::Descend, BinaryOpcode::Call),
            (PlanOp::Ascend, BinaryOpcode::Ret),
            (PlanOp::Swarm, BinaryOpcode::Fork),
            (PlanOp::Merge, BinaryOpcode::Merge),
            (PlanOp::Escalate, BinaryOpcode::Act),
        ];
        for (plan_op, expected) in cases {
            let mut packet = SigmaPacket::new();
            packet.plan.operators.push(plan_op);
            let encoded = encode_instruction(&packet).unwrap();
            let header = encoded[0];
            let opcode_bits = (header >> 2) & 0x0F;
            assert_eq!(
                opcode_bits, expected as u8,
                "plan op {:?} should map to opcode {:?}",
                plan_op, expected
            );
        }
    }

    #[test]
    fn test_encode_cancel_maps_to_halt() {
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Cancel);
        let encoded = encode_instruction(&packet).unwrap();
        let header = encoded[0];
        let opcode_bits = (header >> 2) & 0x0F;
        assert_eq!(opcode_bits, BinaryOpcode::Halt as u8);
    }
}
