// See plans/01-sigma-language.md §9

use crate::error::ParseError;
use crate::packet::{ContextField, DataField, IntentField, PlanField, SigmaPacket};
use crate::token::{BoundaryKind, Token};
use a2x_core::ProtocolId;

/// Parse a single Σ∞ packet from a token slice.
///
/// Returns the parsed packet and the number of tokens consumed.
///
/// Expected format:
/// ⟦ Σ∞ ⟧ ⟬ I:op... ∷ C:op...⟨label⟩ ∷ P:op... ∷ D:op... ⟭
fn parse_one_packet(tokens: &[Token]) -> Result<(SigmaPacket, usize), ParseError> {
    let mut packet = SigmaPacket::new();
    let mut i = 0;

    // 1. Opening boundary ⟦ or ⟬
    match tokens.get(i) {
        Some(Token::Boundary(BoundaryKind::Open)) => {
            i += 1;
        }
        Some(other) => {
            return Err(ParseError::ExpectedToken {
                pos: i,
                expected: "Boundary(Open)".into(),
                found: other.clone(),
            });
        }
        None => return Err(ParseError::UnterminatedInstruction),
    }

    // 2. Protocol identifier Σ∞
    match tokens.get(i) {
        Some(Token::ProtocolId) => {
            packet.protocol = ProtocolId::Sigma;
            i += 1;
        }
        Some(other) => {
            return Err(ParseError::ExpectedToken {
                pos: i,
                expected: "ProtocolId".into(),
                found: other.clone(),
            });
        }
        None => return Err(ParseError::UnterminatedInstruction),
    }

    // 3. Closing boundary ⟧
    match tokens.get(i) {
        Some(Token::Boundary(BoundaryKind::Close)) => {
            i += 1;
        }
        Some(other) => {
            return Err(ParseError::ExpectedToken {
                pos: i,
                expected: "Boundary(Close)".into(),
                found: other.clone(),
            });
        }
        None => return Err(ParseError::UnterminatedInstruction),
    }

    // 4. Opening context boundary ⟬
    match tokens.get(i) {
        Some(Token::Boundary(BoundaryKind::Open)) => {
            i += 1;
        }
        Some(other) => {
            return Err(ParseError::ExpectedToken {
                pos: i,
                expected: "Boundary(Open)".into(),
                found: other.clone(),
            });
        }
        None => return Err(ParseError::UnterminatedInstruction),
    }

    // 5. Field parsing loop: I:, C:, P:, D: separated by ∷
    while i < tokens.len() {
        match tokens.get(i) {
            Some(Token::Boundary(BoundaryKind::Close)) => {
                // ⟭ — end of instruction
                i += 1;
                break;
            }
            Some(Token::Label(field_name)) => {
                let field_char = field_name.as_str();
                i += 1;

                // Expect colon
                match tokens.get(i) {
                    Some(Token::Label(s)) if s == ":" => {
                        i += 1;
                    }
                    _ => {}
                }

                // Parse field content: operators + labels
                let (fields, labels, consumed) = parse_field_content(&tokens[i..])?;
                i += consumed;

                match field_char {
                    "I" => {
                        packet.intent = IntentField {
                            operators: fields
                                .iter()
                                .filter_map(|t| match t {
                                    Token::IntentOp(op) => Some(*op),
                                    _ => None,
                                })
                                .collect(),
                        };
                    }
                    "C" => {
                        packet.context = ContextField {
                            operators: fields
                                .iter()
                                .filter_map(|t| match t {
                                    Token::ContextOp(op) => Some(*op),
                                    _ => None,
                                })
                                .collect(),
                            labels,
                        };
                    }
                    "P" => {
                        packet.plan = PlanField {
                            operators: fields
                                .iter()
                                .filter_map(|t| match t {
                                    Token::PlanOp(op) => Some(*op),
                                    _ => None,
                                })
                                .collect(),
                        };
                    }
                    "D" => {
                        packet.data = DataField {
                            operators: fields
                                .iter()
                                .filter_map(|t| match t {
                                    Token::DataOp(op) => Some(*op),
                                    _ => None,
                                })
                                .collect(),
                            payload: Vec::new(),
                        };
                    }
                    _ => {
                        return Err(ParseError::UnexpectedLabel {
                            pos: i,
                            label: field_char.to_string(),
                        });
                    }
                }
            }
            Some(Token::FieldSeparator) => {
                i += 1;
            }
            Some(other) => {
                return Err(ParseError::UnexpectedToken {
                    pos: i,
                    token: other.clone(),
                });
            }
            None => return Err(ParseError::UnterminatedInstruction),
        }
    }

    Ok((packet, i))
}

/// Parse field content — collect operators and labels until next ∷ or boundary.
fn parse_field_content(tokens: &[Token]) -> Result<(Vec<Token>, Vec<String>, usize), ParseError> {
    let mut fields = Vec::new();
    let mut labels = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::FieldSeparator => break,
            Token::Boundary(_) => break,
            Token::ProtocolId => break,
            Token::Label(s)
                if (s == "I" || s == "C" || s == "P" || s == "D")
                    && matches!(tokens.get(i + 1), Some(Token::Label(colon)) if colon == ":") =>
            {
                // New field prefix (e.g. "I:") — stop here
                break;
            }
            Token::Label(s) => {
                labels.push(s.clone());
                i += 1;
            }
            other => {
                fields.push(other.clone());
                i += 1;
            }
        }
    }

    Ok((fields, labels, i))
}

/// Parse a full sequence of tokens into a vector of SigmaPackets.
pub fn parse(tokens: &[Token]) -> Result<Vec<SigmaPacket>, ParseError> {
    let mut packets = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Boundary(BoundaryKind::Open) => {
                let (packet, consumed) = parse_one_packet(&tokens[i..])?;
                packets.push(packet);
                i += consumed;
            }
            Token::Label(_name) => {
                // Labels outside packets are label definitions for jump targets.
                // For now, skip them; they'll be handled at the SigmaProgram level.
                i += 1;
                // Skip colon if present
                if i < tokens.len() && matches!(&tokens[i], Token::Label(s) if s == ":") {
                    i += 1;
                }
            }
            other => {
                return Err(ParseError::UnexpectedToken {
                    pos: i,
                    token: other.clone(),
                });
            }
        }
    }

    Ok(packets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextOp;
    use crate::data::DataOp;
    use crate::intent::IntentOp;
    use crate::plan::PlanOp;
    use crate::tokenizer::lex;

    #[test]
    fn test_parse_simple_packet() {
        let input = "⟦Σ∞⟧⟬I:⚡ ∷ C:⟨sys⟩ ∷ P:⥂ ∷ D:⌬⟭";
        let tokens = lex(input).unwrap();
        let packets = parse(&tokens).unwrap();
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(pkt.intent.operators, vec![IntentOp::Lightning]);
        assert_eq!(pkt.context.labels, vec!["sys"]);
        assert_eq!(pkt.plan.operators, vec![PlanOp::Sequential]);
        assert_eq!(pkt.data.operators, vec![DataOp::RawTensor]);
    }

    #[test]
    fn test_parse_anomaly_scan_packet() {
        let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
        let tokens = lex(input).unwrap();
        let packets = parse(&tokens).unwrap();
        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(
            pkt.intent.operators,
            vec![IntentOp::Lightning, IntentOp::Synthesis, IntentOp::Parallel,]
        );
        assert_eq!(
            pkt.context.operators,
            vec![ContextOp::Compression, ContextOp::Uncertainty,]
        );
        assert_eq!(pkt.context.labels, vec!["sys"]);
        assert_eq!(
            pkt.plan.operators,
            vec![PlanOp::Swarm, PlanOp::Enforce, PlanOp::Descend,]
        );
        assert_eq!(
            pkt.data.operators,
            vec![DataOp::GraphDelta, DataOp::Summary, DataOp::Fusion,]
        );
    }

    #[test]
    fn test_parse_multiple_packets() {
        let input = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨scope⟩ ∷ P:⥂ ∷ D:⌵⟭⟦Σ∞⟧⟬I:✕ ∷ C:⟘ ∷ P:⤉ ∷ D:⟘⟭";
        let tokens = lex(input).unwrap();
        let packets = parse(&tokens).unwrap();
        assert_eq!(packets.len(), 2);
    }
}
