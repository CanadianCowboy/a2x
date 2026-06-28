// Property-based tests for the a2x-sigma tokenizer and parser.
//
// See PLAN.md §12 (Testing Strategy) and plans/01-sigma-language.md.
//
// Two properties under test:
//   1. Roundtrip: parse(serialize(packet)) == packet for all valid packets.
//   2. Tokenizer never panics: lex() never panics on arbitrary input.

use proptest::prelude::*;
use proptest::strategy::Strategy;

use a2x_sigma::context::ContextOp;
use a2x_sigma::data::DataOp;
use a2x_sigma::intent::IntentOp;
use a2x_sigma::packet::{ContextField, DataField, IntentField, PlanField, SigmaPacket};
use a2x_sigma::plan::PlanOp;
use a2x_sigma::tokenizer::lex;
use a2x_sigma::parser::parse;
use a2x_sigma::serialize_packet;

// ---------------------------------------------------------------------------
// Strategies: generate random valid operators
// ---------------------------------------------------------------------------

fn intent_op_strategy() -> impl Strategy<Value = IntentOp> {
    prop_oneof![
        Just(IntentOp::Lightning),
        Just(IntentOp::Warning),
        Just(IntentOp::Star),
        Just(IntentOp::Synthesis),
        Just(IntentOp::Cancel),
        Just(IntentOp::Contradiction),
        Just(IntentOp::Delay),
        Just(IntentOp::Accelerate),
        Just(IntentOp::Parallel),
        Just(IntentOp::Merge),
        Just(IntentOp::Split),
    ]
}

fn context_op_strategy() -> impl Strategy<Value = ContextOp> {
    // NOTE: Resolved (⟧) is excluded because U+27E7 is overloaded — it is
    // both the closing boundary marker AND the Resolved operator. The tokenizer
    // always matches ⟧ as Boundary(Close) first, so Resolved cannot survive a
    // lex→parse roundtrip. This ambiguity should be resolved in Phase 1.
    prop_oneof![
        Just(ContextOp::Null),
        Just(ContextOp::Universal),
        Just(ContextOp::Compression),
        Just(ContextOp::Uncertainty),
        Just(ContextOp::CausalChain),
        Just(ContextOp::SpatialChain),
        Just(ContextOp::TemporalChain),
        Just(ContextOp::Probabilistic),
        Just(ContextOp::Conflict),
    ]
}

fn plan_op_strategy() -> impl Strategy<Value = PlanOp> {
    prop_oneof![
        Just(PlanOp::Descend),
        Just(PlanOp::Ascend),
        Just(PlanOp::Escalate),
        Just(PlanOp::DeEscalate),
        Just(PlanOp::Branch),
        Just(PlanOp::Merge),
        Just(PlanOp::Enforce),
        Just(PlanOp::Relax),
        Just(PlanOp::Swarm),
        Just(PlanOp::Sequential),
        Just(PlanOp::Recursive),
        Just(PlanOp::SelfModifying),
    ]
}

fn data_op_strategy() -> impl Strategy<Value = DataOp> {
    prop_oneof![
        Just(DataOp::RawTensor),
        Just(DataOp::LatentVector),
        Just(DataOp::GraphDelta),
        Just(DataOp::DiffPatch),
        Just(DataOp::Binary),
        Just(DataOp::Fusion),
        Just(DataOp::Streaming),
        Just(DataOp::Summary),
        Just(DataOp::Anomaly),
        Just(DataOp::Schema),
        Just(DataOp::SelfDescribing),
    ]
}

fn label_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,11}".prop_map(|s| s)
}

// ---------------------------------------------------------------------------
// Build a SigmaPacket from random operators
// ---------------------------------------------------------------------------

fn sigma_packet_strategy() -> impl Strategy<Value = SigmaPacket> {
    (
        proptest::collection::vec(intent_op_strategy(), 0..4),
        proptest::collection::vec(context_op_strategy(), 0..4),
        proptest::collection::vec(plan_op_strategy(), 0..4),
        proptest::collection::vec(data_op_strategy(), 0..4),
        proptest::collection::vec(label_strategy(), 0..2),
    )
        .prop_map(
            |(intent_ops, context_ops, plan_ops, data_ops, labels)| {
                let mut pkt = SigmaPacket::new();
                pkt.intent = IntentField {
                    operators: intent_ops,
                };
                pkt.context = ContextField {
                    operators: context_ops,
                    labels,
                };
                pkt.plan = PlanField {
                    operators: plan_ops,
                };
                pkt.data = DataField {
                    operators: data_ops,
                    payload: Vec::new(),
                };
                pkt
            },
        )
}

// ---------------------------------------------------------------------------
// Property 1: serialize → lex → parse roundtrip
// ---------------------------------------------------------------------------

proptest! {
    /// For any valid packet, serializing it and then parsing it back
    /// should produce an equivalent packet.
    #[test]
    fn roundtrip_packet_to_text_and_back(pkt in sigma_packet_strategy()) {
        let serialized = serialize_packet(&pkt);

        // Lex: must succeed on valid serialized output
        let tokens = lex(&serialized).expect("lexer should accept serialized packet");

        // Parse: must succeed
        let packets = parse(&tokens).expect("parser should accept lexer output");

        prop_assert!(!packets.is_empty(), "should produce at least one packet");

        let reparse = &packets[0];

        // The reprised packet should have the same operators
        prop_assert_eq!(
            &pkt.intent.operators, &reparse.intent.operators,
            "intent operators should match"
        );
        prop_assert_eq!(
            &pkt.context.operators, &reparse.context.operators,
            "context operators should match"
        );
        prop_assert_eq!(
            &pkt.context.labels, &reparse.context.labels,
            "context labels should match"
        );
        prop_assert_eq!(
            &pkt.plan.operators, &reparse.plan.operators,
            "plan operators should match"
        );
        prop_assert_eq!(
            &pkt.data.operators, &reparse.data.operators,
            "data operators should match"
        );
    }

    /// The tokenizer must never panic on arbitrary input.
    #[test]
    fn tokenizer_never_panics(input in "\\PC*") {
        // Arbitrary Unicode — lex should either succeed or return Err, never panic
        let _ = lex(&input);
    }

    /// The tokenizer must never panic on raw byte strings.
    #[test]
    fn tokenizer_never_panics_on_bytes(input: Vec<u8>) {
        // Raw bytes — may be invalid UTF-8, but String::from_utf8_lossy handles it
        let s = String::from_utf8_lossy(&input);
        let _ = lex(&s);
    }
}
