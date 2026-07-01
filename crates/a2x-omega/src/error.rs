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
    /// Semantic analysis failed (Stage 3).
    SemanticError(SemanticError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnsupportedOpcode(op) => write!(f, "unsupported opcode: {}", op),
            CompileError::EmptyProgram => write!(f, "cannot compile empty program"),
            CompileError::IrError(msg) => write!(f, "IR error: {}", msg),
            CompileError::SemanticError(e) => write!(f, "semantic error: {}", e),
        }
    }
}

/// Semantic analysis errors (Stage 3 of the compiler pipeline).
/// See plans/02-omega-compiler.md §3.
#[derive(Clone, Debug, PartialEq)]
pub enum SemanticError {
    /// Jump target references a label that doesn't exist.
    UndefinedLabel {
        label: String,
        instruction_index: usize,
    },
    /// Sub-program name is referenced but never defined.
    UndefinedSubProgram {
        name: String,
        instruction_index: usize,
    },
    /// Contradictory operators in the same instruction.
    ContradictoryOperators {
        instruction_index: usize,
        operators: String,
    },
    /// Data field type doesn't match expected type for the opcode.
    TypeMismatch {
        instruction_index: usize,
        expected: String,
        found: String,
    },
    /// Instruction has no intent operators (empty I field).
    EmptyIntent { instruction_index: usize },
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemanticError::UndefinedLabel {
                label,
                instruction_index,
            } => {
                write!(
                    f,
                    "undefined label '{}' at instruction {}",
                    label, instruction_index
                )
            }
            SemanticError::UndefinedSubProgram {
                name,
                instruction_index,
            } => {
                write!(
                    f,
                    "undefined sub-program '{}' at instruction {}",
                    name, instruction_index
                )
            }
            SemanticError::ContradictoryOperators {
                instruction_index,
                operators,
            } => {
                write!(
                    f,
                    "contradictory operators [{}] at instruction {}",
                    operators, instruction_index
                )
            }
            SemanticError::TypeMismatch {
                instruction_index,
                expected,
                found,
            } => {
                write!(
                    f,
                    "type mismatch at instruction {}: expected {}, found {}",
                    instruction_index, expected, found
                )
            }
            SemanticError::EmptyIntent { instruction_index } => {
                write!(f, "empty intent field at instruction {}", instruction_index)
            }
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
