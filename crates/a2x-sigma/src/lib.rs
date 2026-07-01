// a2x-sigma — Σ∞ symbolic programming language / ISA
// See plans/01-sigma-language.md
//
// Provides: tokenizer, parser, operator tables, SigmaPacket, SigmaProgram

pub mod binary;
pub mod context;
pub mod data;
pub mod display;
pub mod error;
pub mod intent;
pub mod packet;
pub mod parser;
pub mod plan;
pub mod program;
pub mod token;
pub mod tokenizer;

// Re-export key types for convenience
pub use binary::{
    decode_instruction, encode_instruction, from_bytes, to_bytes, BinaryError, BinaryOpcode,
};
pub use context::ContextOp;
pub use data::DataOp;
pub use error::{LexError, ParseError};
pub use intent::IntentOp;
pub use packet::{ContextField, DataField, IntentField, PlanField, SigmaPacket};
pub use parser::parse;
pub use plan::PlanOp;
pub use program::{ProgramMetadata, ProgramRef, SigmaProgram};
pub use token::{BoundaryKind, Token};
pub use tokenizer::lex;

/// Parse a Σ∞ source string directly into a SigmaProgram.
///
/// Convenience function that combines lexing and parsing.
pub fn parse_program(input: &str) -> Result<SigmaProgram, Box<dyn std::error::Error>> {
    let tokens = lex(input).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let packets = parse(&tokens).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let mut program = SigmaProgram::new();
    for pkt in packets {
        program.push(pkt);
    }
    Ok(program)
}

/// Serialize a SigmaPacket back to its text representation.
pub fn serialize_packet(packet: &SigmaPacket) -> String {
    packet.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_program_roundtrip() {
        let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
        let program = parse_program(input).unwrap();
        assert_eq!(program.len(), 1);
        let serialized = serialize_packet(&program.instructions[0]);
        // The serialized form should be semantically equivalent
        assert!(serialized.contains("⚡✣⩫"));
        assert!(serialized.contains("⟨sys⟩"));
    }
}
