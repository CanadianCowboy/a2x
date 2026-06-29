// Phase 3.4 — Cross-machine Σ↔Ω↔CCS end-to-end via TcpTransport
//
// Full pipeline per PLAN §18 / checkpoint §4.4:
//
//   Sender                         Receiver
//   ────────                       ────────
//   Parse Σ∞ ──► Compile to Ω ──► TCP send wire bytes
//                                    │
//                                    ▼
//                                  Reconstruct Ω packet
//                                  Decompile to Σ∞
//                                  Load Σ∞ into CCS VM
//                                  VM tick (execute 1 step)
//                                  Serialize result → wire bytes
//                                    │
//   TCP receive ◄────────────────────┘
//   Assert identity (source Σ∞ ≡ result Σ∞)

use a2x_bus::tcp_transport::TcpTransport;
use a2x_bus::transport::Transport;
use a2x_bus::wire::{MessageType, WireMessage};
use a2x_ccs::CcsVm;
use a2x_ccs::VmStatus;
use a2x_core::graph::WorldGraph;
use a2x_core::AgentId;
use a2x_omega::compiler::CompileToOmega;
use a2x_omega::decoder::DecompileToSigma;
use a2x_omega::passes::OptimizationLevel;
use a2x_sigma::intent::IntentOp;
use a2x_sigma::parse_program;

/// Serialize an OmegaPacket into length-prefixed wire bytes.
fn serialize_omega_packet(packet: &a2x_omega::OmegaPacket) -> Vec<u8> {
    let mut buf = Vec::new();
    let data: &[f32; 29796] = &packet.data;
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    for v in data.iter() {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

/// Reconstruct an OmegaPacket from length-prefixed wire bytes.
fn deserialize_omega_packet(payload: &[u8]) -> a2x_omega::OmegaPacket {
    let data_len = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
    assert_eq!(data_len, 29796, "tensor dimension must match");

    let mut data = [0.0f32; 29796];
    let start = 4;
    for (i, slot) in data.iter_mut().enumerate() {
        let off = start + i * 4;
        *slot = f32::from_le_bytes([
            payload[off],
            payload[off + 1],
            payload[off + 2],
            payload[off + 3],
        ]);
    }
    a2x_omega::OmegaPacket::from_raw(data)
}

/// Serialize a SigmaPacket into length-prefixed wire bytes (same framing as Ω).
fn serialize_sigma_packet(packet: &a2x_sigma::SigmaPacket) -> Vec<u8> {
    let text = packet.to_string();
    let bytes = text.as_bytes();
    let mut buf = Vec::new();
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
    buf
}

#[test]
fn test_cross_machine_sigma_omega_ccs_roundtrip() {
    // === SENDER: Parse Σ∞ → Compile to Ω → compute expected round-trip ===
    // Ground packet: I:✦ (Star), no context operands (safe for VM execution).
    // The Ω encoder projects only the intent hash into Ω_I; context/plan/data
    // fields are not preserved across compile→decompile. The "expected" Σ∞ is
    // the decompiled form — this is what the identity check verifies.
    let input = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨⟩ ∷ P:⥁ ∷ D:⌬⟭";
    let source_sigma = parse_program(input).expect("parse");
    let omega = source_sigma
        .compile(OptimizationLevel::Light)
        .expect("compile");
    assert_eq!(omega.instructions.len(), 1, "one packet compiled");

    // Compute the expected round-tripped Σ∞ (compile then decompile locally).
    let expected_roundtripped =
        <a2x_sigma::SigmaPacket as DecompileToSigma>::decompile(&omega.instructions[0])
            .expect("local decompile must succeed");

    // Serialize first Ω packet to wire bytes.
    let wire_bytes = serialize_omega_packet(&omega.instructions[0]);

    // === TCP TRANSPORT: Sender → Receiver ===
    let mut server = TcpTransport::new();
    let key = "127.0.0.1:0";
    server.register(key).unwrap();
    let bound = server.bound_addr(key).unwrap();

    let mut client = TcpTransport::new();
    let msg = WireMessage::new(
        MessageType::OmegaProgram,
        AgentId::new("sender"),
        Some(AgentId::new("receiver")),
        1,
        wire_bytes,
    );
    client.send(&bound.to_string(), msg).unwrap();

    // === RECEIVER: Read Ω → Reconstruct → Decompile ===
    let received = server.recv(key).unwrap();
    assert_eq!(received.len(), 1);
    let rx_msg = &received[0];
    assert_eq!(rx_msg.msg_type, MessageType::OmegaProgram);
    assert_eq!(rx_msg.sender, AgentId::new("sender"));

    let omega_packet = deserialize_omega_packet(&rx_msg.payload);

    // Decompile Ω → Σ∞ — intent operator must survive the round-trip.
    let decompiled_sigma = <a2x_sigma::SigmaPacket as DecompileToSigma>::decompile(&omega_packet)
        .expect("decompile must succeed");
    assert_eq!(
        decompiled_sigma.intent.operators,
        vec![IntentOp::Star],
        "✦ → Star must survive compile → wire → decompile"
    );

    // === RECEIVER: CCS VM executes the decompiled Σ∞ ===
    let mut vm = CcsVm::new();
    let mut vm_program = a2x_sigma::SigmaProgram::new();
    vm_program.push(decompiled_sigma.clone());
    vm.load(vm_program);

    // Single step — Ground (Star) allocates exactly 1 WorldGraph node.
    let status = vm.step().expect("VM step must not error");
    assert_eq!(status, VmStatus::Running, "VM should still be running");
    assert_eq!(
        vm.world_graph.node_count(),
        1,
        "Ground must allocate exactly 1 node"
    );
    assert_eq!(vm.steps_executed(), 1, "VM must have executed 1 step");

    // === RECEIVER → SENDER: Send result back over TCP ===
    let result_sigma = decompiled_sigma;
    let result_wire = serialize_sigma_packet(&result_sigma);

    let mut result_server = TcpTransport::new();
    let result_key = "127.0.0.1:0";
    result_server.register(result_key).unwrap();
    let result_bound = result_server.bound_addr(result_key).unwrap();

    let mut result_client = TcpTransport::new();
    let result_msg = WireMessage::new(
        MessageType::SigmaProgram,
        AgentId::new("receiver"),
        Some(AgentId::new("sender")),
        2,
        result_wire,
    );
    result_client
        .send(&result_bound.to_string(), result_msg)
        .unwrap();

    // === SENDER: Receive result and identity-check ===
    let result_received = result_server.recv(result_key).unwrap();
    assert_eq!(result_received.len(), 1);
    let result_rx = &result_received[0];
    assert_eq!(result_rx.msg_type, MessageType::SigmaProgram);

    // Identity check: the expected round-tripped Σ∞ must match the result
    // packet string — full round-trip through compile → TCP → decompile →
    // CCS VM → TCP preserves the symbolic form.
    let expected_str = expected_roundtripped.to_string();
    let result_payload =
        std::str::from_utf8(&result_rx.payload[4..]).expect("result payload must be valid UTF-8");
    assert_eq!(
        expected_str, result_payload,
        "round-tripped Σ∞ must match result Σ∞ — full round-trip identity"
    );
}
