// See plans/01-sigma-language.md §8

use crate::context::ContextOp;
use crate::data::DataOp;
use crate::intent::IntentOp;
use crate::plan::PlanOp;
use crate::token::{BoundaryKind, Token};

/// Error from the lexer/tokenizer.
#[derive(Clone, Debug, PartialEq)]
pub enum LexError {
    /// Unrecognized character in input.
    UnknownCharacter(char),
    /// Σ followed by something other than ∞.
    InvalidProtocolId(char),
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::UnknownCharacter(c) => write!(f, "unknown character: '{}'", c),
            LexError::InvalidProtocolId(c) => write!(f, "expected '∞' after 'Σ', got '{}'", c),
        }
    }
}

impl std::error::Error for LexError {}

/// Tokenize (lex) a Σ∞ source string into a vector of tokens.
///
/// The lexer matches Unicode special characters against the operator tables.
/// Whitespace between tokens is ignored. Labels in angle brackets ⟨text⟩ are
/// collected as single tokens.
///
/// # Examples
///
/// ```ignore
/// let tokens = lex("⟦Σ∞⟧").unwrap();
/// assert_eq!(tokens.len(), 4);
/// ```
pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Skip whitespace
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        let token = match c {
            // Protocol identifier: Σ∞ (multi-char)
            'Σ' => {
                if i + 1 < chars.len() && chars[i + 1] == '∞' {
                    i += 2;
                    Token::ProtocolId
                } else {
                    let next = chars.get(i + 1).copied().unwrap_or('\0');
                    return Err(LexError::InvalidProtocolId(next));
                }
            }

            // Boundary markers (both open markers map to Boundary(Open))
            '⟦' | '⟬' => {
                i += 1;
                Token::Boundary(BoundaryKind::Open)
            }
            '⟧' | '⟭' => {
                i += 1;
                Token::Boundary(BoundaryKind::Close)
            }

            // Field separator
            '∷' => {
                i += 1;
                Token::FieldSeparator
            }

            // Angle brackets — collect label text
            '⟨' => {
                i += 1; // skip opening ⟨
                let mut label = String::new();
                while i < chars.len() && chars[i] != '⟩' {
                    label.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip closing ⟩
                }
                Token::Label(label)
            }

            // Try intent operators
            _ if IntentOp::from_char(c).is_some() => {
                i += 1;
                Token::IntentOp(IntentOp::from_char(c).unwrap())
            }

            // Try context operators
            _ if ContextOp::from_char(c).is_some() => {
                i += 1;
                Token::ContextOp(ContextOp::from_char(c).unwrap())
            }

            // Try plan operators
            _ if PlanOp::from_char(c).is_some() => {
                i += 1;
                Token::PlanOp(PlanOp::from_char(c).unwrap())
            }

            // Try data operators
            _ if DataOp::from_char(c).is_some() => {
                i += 1;
                Token::DataOp(DataOp::from_char(c).unwrap())
            }

            // Alphanumeric label (not in angle brackets — e.g., standalone identifiers)
            // NOTE: colon (:) is NOT included here — it's handled as a separate token
            c if c.is_alphanumeric() || c == '_' || c == '-' || c == ',' => {
                let mut label = String::new();
                while i < chars.len() {
                    let nc = chars[i];
                    if nc.is_alphanumeric()
                        || nc == '_'
                        || nc == '-'
                        || nc == ','
                        || nc == '.'
                        || nc == '?'
                        || nc == '*'
                        || nc == '#'
                    {
                        label.push(nc);
                        i += 1;
                    } else {
                        break;
                    }
                }
                Token::Label(label)
            }

            // Colon (field prefix like I:, C:, P:, D:) — tokenized separately
            ':' => {
                i += 1;
                Token::Label(":".to_string())
            }

            _ => return Err(LexError::UnknownCharacter(c)),
        };

        tokens.push(token);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_empty() {
        let tokens = lex("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_lex_whitespace_only() {
        let tokens = lex("  \n  ").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_lex_protocol_id() {
        let tokens = lex("Σ∞").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::ProtocolId);
    }

    #[test]
    fn test_lex_boundaries() {
        let tokens = lex("⟦⟧⟬⟭").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], Token::Boundary(BoundaryKind::Open)); // ⟦
        assert_eq!(tokens[1], Token::Boundary(BoundaryKind::Close)); // ⟧
        assert_eq!(tokens[2], Token::Boundary(BoundaryKind::Open)); // ⟬
        assert_eq!(tokens[3], Token::Boundary(BoundaryKind::Close)); // ⟭
    }

    #[test]
    fn test_lex_field_separator() {
        let tokens = lex("∷").unwrap();
        assert_eq!(tokens[0], Token::FieldSeparator);
    }

    #[test]
    fn test_lex_intent_ops() {
        let tokens = lex("⚡✦✣").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::IntentOp(IntentOp::Lightning));
        assert_eq!(tokens[1], Token::IntentOp(IntentOp::Star));
        assert_eq!(tokens[2], Token::IntentOp(IntentOp::Synthesis));
    }

    #[test]
    fn test_lex_context_ops() {
        let tokens = lex("⟚⟞").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::ContextOp(ContextOp::Compression));
        assert_eq!(tokens[1], Token::ContextOp(ContextOp::Uncertainty));
    }

    #[test]
    fn test_lex_plan_ops() {
        let tokens = lex("⥁⤒⤈").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::PlanOp(PlanOp::Swarm));
        assert_eq!(tokens[1], Token::PlanOp(PlanOp::Enforce));
        assert_eq!(tokens[2], Token::PlanOp(PlanOp::Descend));
    }

    #[test]
    fn test_lex_data_ops() {
        let tokens = lex("⌮⌳⌱").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::DataOp(DataOp::GraphDelta));
        assert_eq!(tokens[1], Token::DataOp(DataOp::Summary));
        assert_eq!(tokens[2], Token::DataOp(DataOp::Fusion));
    }

    #[test]
    fn test_lex_label_in_brackets() {
        let tokens = lex("⟨sys⟩").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Label("sys".to_string()));
    }

    #[test]
    fn test_lex_full_packet() {
        let input = "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";
        let tokens = lex(input).unwrap();
        // Should have many tokens
        assert!(!tokens.is_empty());
        // First few should be boundaries + protocol
        assert_eq!(tokens[0], Token::Boundary(BoundaryKind::Open));
        assert_eq!(tokens[1], Token::ProtocolId);
        assert_eq!(tokens[2], Token::Boundary(BoundaryKind::Close));
        assert_eq!(tokens[3], Token::Boundary(BoundaryKind::Open));
    }

    #[test]
    fn test_lex_unknown_character() {
        let result = lex("hello");
        assert!(result.is_ok()); // alphanumeric is fine
        let result = lex("§");
        assert!(result.is_err());
    }
}
