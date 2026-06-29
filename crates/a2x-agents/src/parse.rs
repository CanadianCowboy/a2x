// Shared utility for parsing Packets into SigmaPrograms and vice-versa.
//
// Extracted from duplicate logic in CliAgent::execute() and Orchestrator::execute().

use a2x_core::error::AgentError;
use a2x_core::packet::Packet;
use a2x_sigma::program::SigmaProgram;

/// Parse a raw Packet into a SigmaProgram.
///
/// If the packet is raw bytes, attempts to interpret them as UTF-8 Sigma text,
/// parse into a SigmaProgram, and return it.
pub fn packet_to_sigma_program(packet: Packet) -> Result<SigmaProgram, AgentError> {
    match packet {
        Packet::Raw(bytes) => {
            let text = String::from_utf8(bytes).map_err(|e| AgentError::ProgramCrash {
                program_id: a2x_core::ProgramId::zero(),
                reason: format!("invalid UTF-8 in packet: {}", e),
            })?;
            a2x_sigma::parse_program(&text).map_err(|e| AgentError::ProgramCrash {
                program_id: a2x_core::ProgramId::zero(),
                reason: format!("parse error: {}", e),
            })
        }
    }
}

/// Serialize a SigmaProgram back into a raw Packet.
///
/// Each instruction is converted to its text form and concatenated.
pub fn sigma_program_to_packet(program: &SigmaProgram) -> Packet {
    let output_text = program
        .instructions
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("");
    Packet::Raw(output_text.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_valid() {
        let text = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨test⟩ ∷ P:⥂ ∷ D:⌵⟭";
        let packet = Packet::Raw(text.as_bytes().to_vec());
        let prog = packet_to_sigma_program(packet).unwrap();
        assert_eq!(prog.len(), 1);
        let back = sigma_program_to_packet(&prog);
        assert!(matches!(back, Packet::Raw(_)));
    }

    #[test]
    fn test_packet_invalid_utf8() {
        let packet = Packet::Raw(vec![0xFF, 0xFE, 0xFD]);
        assert!(packet_to_sigma_program(packet).is_err());
    }

    #[test]
    fn test_packet_invalid_sigma() {
        // Malformed Sigma: has boundaries but missing protocol identifier
        let packet = Packet::Raw("⟦not valid sigma⟧".as_bytes().to_vec());
        assert!(packet_to_sigma_program(packet).is_err());
    }

    #[test]
    fn test_empty_program_roundtrip() {
        let packet = Packet::Raw(vec![]);
        let prog = packet_to_sigma_program(packet).unwrap();
        assert!(prog.is_empty());
    }
}
