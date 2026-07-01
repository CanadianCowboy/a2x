// Semantic analyzer — Stage 3 of the Omega compilation pipeline.
// See plans/02-omega-compiler.md §3.
//
// Validates a Σ∞ program for semantic correctness before IR generation:
//   1. All jump/call targets reference valid labels or sub-programs.
//   2. No contradictory operators in the same instruction.
//   3. Data field types match expected types for the opcode.
//   4. Every instruction has a valid intent.
//
// Fixes T2-1 from the comprehensive audit: the semantic analyzer was
// a pure stub with "no validation code".

use a2x_sigma::program::SigmaProgram;
use a2x_sigma::IntentOp;
use a2x_sigma::PlanOp;
use a2x_sigma::SigmaPacket;

use crate::error::{CompileError, SemanticError};

/// Run semantic analysis on a Σ∞ program.
///
/// Returns `Ok(())` if the program is semantically valid, or a
/// `CompileError::SemanticError` describing the first violation found.
pub fn analyze(program: &SigmaProgram) -> Result<(), CompileError> {
    if program.is_empty() {
        return Err(CompileError::EmptyProgram);
    }

    for (i, packet) in program.instructions.iter().enumerate() {
        validate_instruction(packet, i, program)?;
    }

    Ok(())
}

/// Validate a single instruction for semantic correctness.
fn validate_instruction(
    packet: &SigmaPacket,
    index: usize,
    program: &SigmaProgram,
) -> Result<(), CompileError> {
    // 1. Every instruction must have at least one intent or plan operator.
    if packet.intent.operators.is_empty() && packet.plan.operators.is_empty() {
        return Err(CompileError::SemanticError(SemanticError::EmptyIntent {
            instruction_index: index,
        }));
    }

    // 2. No contradictory operators.
    check_contradictions(packet, index)?;

    // 3. Jump/call targets must resolve (only for Branch and Descend).
    //    Swarm (⥁) C-field labels are runtime operands, not compile-time
    //    targets — the VM resolves them from the WorldGraph at runtime.
    check_jump_targets(packet, index, program)?;

    // 4. Data type compatibility.
    check_data_types(packet, index)?;

    Ok(())
}

/// Check for contradictory operator pairs in a single instruction.
///
/// Examples of contradictions:
///   - Cancel (✕) + Synthesis (✣) — can't create and destroy simultaneously.
///   - Descend (⤈) + Ascend (⤉) — can't enter and exit a sub-plan.
///   - Branch (⤐) + Merge (⤑) — can't fork and join at once.
fn check_contradictions(packet: &SigmaPacket, index: usize) -> Result<(), CompileError> {
    // Cancel vs Synthesis
    let has_cancel = packet.intent.operators.contains(&IntentOp::Cancel);
    let has_synthesis = packet.intent.operators.contains(&IntentOp::Synthesis);
    if has_cancel && has_synthesis {
        let ops = format!("{:?} + {:?}", IntentOp::Cancel, IntentOp::Synthesis);
        return Err(CompileError::SemanticError(
            SemanticError::ContradictoryOperators {
                instruction_index: index,
                operators: ops,
            },
        ));
    }

    // Descend vs Ascend
    let has_descend = packet.plan.operators.contains(&PlanOp::Descend);
    let has_ascend = packet.plan.operators.contains(&PlanOp::Ascend);
    if has_descend && has_ascend {
        let ops = format!("{:?} + {:?}", PlanOp::Descend, PlanOp::Ascend);
        return Err(CompileError::SemanticError(
            SemanticError::ContradictoryOperators {
                instruction_index: index,
                operators: ops,
            },
        ));
    }

    // Branch vs Merge
    let has_branch = packet.plan.operators.contains(&PlanOp::Branch);
    let has_merge = packet.plan.operators.contains(&PlanOp::Merge);
    if has_branch && has_merge {
        let ops = format!("{:?} + {:?}", PlanOp::Branch, PlanOp::Merge);
        return Err(CompileError::SemanticError(
            SemanticError::ContradictoryOperators {
                instruction_index: index,
                operators: ops,
            },
        ));
    }

    Ok(())
}

/// Verify that jump/call targets reference valid labels.
///
/// Only Branch (⤐) and Descend (⤈) operators require validated compile-time
/// targets. Swarm (⥁) labels are runtime operands resolved from the
/// WorldGraph — they are NOT validated here.
///
/// Labels in C-field for data operators (Star, Synthesis, etc.) are also
/// runtime operands and are not validated.
fn check_jump_targets(
    packet: &SigmaPacket,
    index: usize,
    program: &SigmaProgram,
) -> Result<(), CompileError> {
    let is_branch = packet.plan.operators.contains(&PlanOp::Branch);
    let is_descend = packet.plan.operators.contains(&PlanOp::Descend);

    if !is_branch && !is_descend {
        return Ok(());
    }

    // Branch and Descend require a C-field label as target.
    if packet.context.labels.is_empty() {
        return Err(CompileError::SemanticError(SemanticError::UndefinedLabel {
            label: "<none>".into(),
            instruction_index: index,
        }));
    }

    for label in &packet.context.labels {
        // Check program labels table
        if program.resolve_label(label).is_some() {
            return Ok(());
        }
        // Check sub-programs
        if program.sub_programs.contains_key(label) {
            return Ok(());
        }
    }
    // None of the context labels resolved to a valid target.
    let label_str = packet
        .context
        .labels
        .first()
        .cloned()
        .unwrap_or_else(|| "<none>".into());
    Err(CompileError::SemanticError(SemanticError::UndefinedLabel {
        label: label_str,
        instruction_index: index,
    }))
}

/// Check that the D (data) field is compatible with the intent opcode.
fn check_data_types(packet: &SigmaPacket, index: usize) -> Result<(), CompileError> {
    // Cancel (✕) with data is suspicious — why cancel AND provide data?
    let has_cancel = packet.intent.operators.contains(&IntentOp::Cancel);
    if has_cancel && (!packet.data.operators.is_empty() || !packet.data.payload.is_empty()) {
        return Err(CompileError::SemanticError(SemanticError::TypeMismatch {
            instruction_index: index,
            expected: "no data for cancel".into(),
            found: "data present".into(),
        }));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_sigma::SigmaPacket;

    fn make_packet_with_intents(intents: &[IntentOp]) -> SigmaPacket {
        let mut p = SigmaPacket::new();
        for op in intents {
            p.intent.operators.push(*op);
        }
        p
    }

    fn make_empty_packet() -> SigmaPacket {
        SigmaPacket::new()
    }

    #[test]
    fn test_analyze_empty_program() {
        let program = SigmaProgram::new();
        let result = analyze(&program);
        assert!(matches!(result, Err(CompileError::EmptyProgram)));
    }

    #[test]
    fn test_analyze_valid_program() {
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Star); // explore
        program.push(packet);
        let result = analyze(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_swarm_with_runtime_label_passes() {
        // Swarm (⥁) with ⟨sys⟩ — labels are runtime operands, not compile-time targets.
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Lightning);
        packet.plan.operators.push(PlanOp::Swarm);
        packet.context.labels.push("sys".into());
        program.push(packet);
        let result = analyze(&program);
        assert!(
            result.is_ok(),
            "swarm labels are runtime operands, not targets"
        );
    }

    #[test]
    fn test_empty_intent_rejected() {
        let mut program = SigmaProgram::new();
        program.push(make_empty_packet());
        let result = analyze(&program);
        assert!(matches!(
            result,
            Err(CompileError::SemanticError(
                SemanticError::EmptyIntent { .. }
            ))
        ));
    }

    #[test]
    fn test_contradictory_cancel_and_synthesis() {
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Cancel);
        packet.intent.operators.push(IntentOp::Synthesis);
        program.push(packet);
        let result = analyze(&program);
        assert!(matches!(
            result,
            Err(CompileError::SemanticError(
                SemanticError::ContradictoryOperators { .. }
            ))
        ));
    }

    #[test]
    fn test_contradictory_descend_and_ascend() {
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Star);
        packet.plan.operators.push(PlanOp::Descend);
        packet.plan.operators.push(PlanOp::Ascend);
        program.push(packet);
        let result = analyze(&program);
        assert!(matches!(
            result,
            Err(CompileError::SemanticError(
                SemanticError::ContradictoryOperators { .. }
            ))
        ));
    }

    #[test]
    fn test_undefined_jump_target() {
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Star);
        packet.plan.operators.push(PlanOp::Branch);
        packet.context.labels.push("missing_label".into());
        program.push(packet);
        let result = analyze(&program);
        assert!(matches!(
            result,
            Err(CompileError::SemanticError(
                SemanticError::UndefinedLabel { .. }
            ))
        ));
    }

    #[test]
    fn test_valid_jump_target() {
        let mut program = SigmaProgram::new();
        program.labels.insert("target".into(), 1);
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Star);
        packet.plan.operators.push(PlanOp::Branch);
        packet.context.labels.push("target".into());
        program.push(packet);
        program.push(make_packet_with_intents(&[IntentOp::Star]));
        let result = analyze(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_with_data_rejected() {
        let mut program = SigmaProgram::new();
        let mut packet = SigmaPacket::new();
        packet.intent.operators.push(IntentOp::Cancel);
        packet.data.payload = vec![1, 2, 3];
        program.push(packet);
        let result = analyze(&program);
        assert!(matches!(
            result,
            Err(CompileError::SemanticError(
                SemanticError::TypeMismatch { .. }
            ))
        ));
    }
}
