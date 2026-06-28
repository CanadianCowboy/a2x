// See plans/01-sigma-language.md §8-9
//
// Centralized error types for the Σ∞ language crate.

use crate::token::Token;

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

/// Error from the parser when assembling tokens into a Σ∞ packet.
#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    /// Encountered an unexpected token at the given position.
    UnexpectedToken { pos: usize, token: Token },
    /// Missing a required field (I, C, P, or D).
    MissingField(&'static str),
    /// Expected a token type that wasn't present at the given position.
    ExpectedToken {
        pos: usize,
        expected: String,
        found: Token,
    },
    /// Unterminated instruction (missing closing boundary).
    UnterminatedInstruction,
    /// A label came at an unexpected position.
    UnexpectedLabel { pos: usize, label: String },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedToken { pos, token } => {
                write!(f, "unexpected token at position {}: {:?}", pos, token)
            }
            ParseError::MissingField(name) => write!(f, "missing required field: {}", name),
            ParseError::ExpectedToken {
                pos,
                expected,
                found,
            } => {
                write!(
                    f,
                    "expected {} at position {}, found {:?}",
                    expected, pos, found
                )
            }
            ParseError::UnterminatedInstruction => write!(f, "unterminated instruction"),
            ParseError::UnexpectedLabel { pos, label } => {
                write!(f, "unexpected label at position {}: {}", pos, label)
            }
        }
    }
}

impl std::error::Error for ParseError {}
