// See plans/02-omega-compiler.md §5
//
// Centralized error types for the Ω compiler crate.

/// Error from the Ω compilation pipeline.
#[derive(Clone, Debug, PartialEq)]
pub enum CompileError {
    /// Unsupported opcode in the compilation target.
    UnsupportedOpcode(String),
    /// The program is empty and cannot be compiled.
    EmptyProgram,
    /// IR generation failed.
    IrError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnsupportedOpcode(op) => write!(f, "unsupported opcode: {}", op),
            CompileError::EmptyProgram => write!(f, "cannot compile empty program"),
            CompileError::IrError(msg) => write!(f, "IR error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

/// Error from the Ω → Σ∞ decompilation process.
#[derive(Clone, Debug, PartialEq)]
pub enum DecompileError {
    /// The tensor does not match any known Σ∞ operator.
    NoMatchingOperator,
    /// The tensor is too small or malformed.
    InvalidTensor(String),
}

impl std::fmt::Display for DecompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecompileError::NoMatchingOperator => write!(f, "no matching Σ∞ operator found"),
            DecompileError::InvalidTensor(msg) => write!(f, "invalid tensor: {}", msg),
        }
    }
}

impl std::error::Error for DecompileError {}
