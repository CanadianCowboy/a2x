// See plans/12-security.md §2 — Agent-to-Agent Security
//
// Ed25519 agent identity, key generation, message signing, and verification.
// This is the cryptographic foundation for bus message authentication.
//
// T4-1: Agent identity with key generation + message signing/verification.

use a2x_core::agent_id::AgentId;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

/// Ed25519-based agent identity.
///
/// Every agent on the bus can have a cryptographic identity. The signing key
/// is kept private; the verifying key is shared during the announce phase so
/// other agents can verify signed messages.
///
/// See plans/12-security.md §2 — "Agent Identity" for the full design.
#[derive(Clone)]
pub struct AgentIdentity {
    /// Unique agent ID.
    pub id: AgentId,
    /// Ed25519 signing key (private — never serialized in plaintext).
    pub signing_key: SigningKey,
    /// Ed25519 verifying key (public — shared on the bus).
    pub verifying_key: VerifyingKey,
}

impl AgentIdentity {
    /// Generate a new random Ed25519 identity for the given agent.
    pub fn generate(id: AgentId) -> Self {
        let mut csprng = OsRng;
        // ed25519-dalek v2: generate a random seed, then create signing key from bytes.
        let mut seed = [0u8; 32];
        use rand::RngCore;
        csprng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        AgentIdentity {
            id,
            signing_key,
            verifying_key,
        }
    }

    /// Sign a message and return the 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Get the verifying key bytes (32 bytes, for wire transport).
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
}

impl std::fmt::Debug for AgentIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vk_hex: String = self
            .verifying_key_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        f.debug_struct("AgentIdentity")
            .field("id", &self.id)
            .field("verifying_key", &vk_hex)
            .finish_non_exhaustive()
    }
}

/// A bus message with Ed25519 signature attached.
///
/// Wraps the serialized message payload with a sender signature for
/// authentication. On first contact, the verifying key is included so
/// the receiver can verify subsequent messages without the key.
///
/// See plans/12-security.md §2 — "Bus Message Signing".
#[derive(Clone, Debug)]
pub struct SignedWireMessage {
    /// Serialized wire message payload.
    pub payload: Vec<u8>,
    /// Ed25519 signature over `payload` (64 bytes).
    pub signature: [u8; 64],
    /// Sender's verifying key (included on first message only).
    pub verifying_key: Option<[u8; 32]>,
}

/// Verify a signed message against the sender's verifying key.
///
/// Returns `true` if the signature is valid for the given payload and key.
pub fn verify_signed_message(
    verifying_key: &VerifyingKey,
    payload: &[u8],
    signature: &[u8; 64],
) -> bool {
    let sig = match Signature::from_slice(signature) {
        Ok(s) => s,
        Err(_) => return false,
    };
    verifying_key.verify_strict(payload, &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{MessageType, WireMessage, WIRE_VERSION};

    #[test]
    fn test_generate_identity() {
        let id = AgentIdentity::generate(AgentId::new("test-agent"));
        assert_eq!(id.id.as_str(), "test-agent");
        let vk_bytes = id.verifying_key_bytes();
        assert_eq!(vk_bytes.len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let identity = AgentIdentity::generate(AgentId::new("signer"));
        let message = "\u{27E6}\u{03A3}\u{221E}\u{27E7}\u{27EC}I:\u{2726} \u{2237} C:\u{27E8}test\u{27E9}\u{27ED}".as_bytes();

        let signature = identity.sign(message);
        let sig_bytes: [u8; 64] = signature.to_bytes();

        assert!(verify_signed_message(
            &identity.verifying_key,
            message,
            &sig_bytes
        ));
    }

    #[test]
    fn test_verify_tampered_message_fails() {
        let identity = AgentIdentity::generate(AgentId::new("signer"));
        let message = b"hello world";
        let signature = identity.sign(message);
        let sig_bytes: [u8; 64] = signature.to_bytes();

        // Tamper with the message
        assert!(!verify_signed_message(
            &identity.verifying_key,
            b"hello WORLD", // different message
            &sig_bytes
        ));
    }

    #[test]
    fn test_different_agents_have_different_keys() {
        let id1 = AgentIdentity::generate(AgentId::new("agent-1"));
        let id2 = AgentIdentity::generate(AgentId::new("agent-2"));

        assert_ne!(id1.verifying_key_bytes(), id2.verifying_key_bytes());

        // Agent 2 can't verify agent 1's signatures
        let msg = b"test";
        let sig1 = id1.sign(msg);
        let sig_bytes: [u8; 64] = sig1.to_bytes();
        assert!(!verify_signed_message(&id2.verifying_key, msg, &sig_bytes));
    }

    #[test]
    fn test_signed_wire_message_roundtrip() {
        let identity = AgentIdentity::generate(AgentId::new("sender"));
        let payload = b"wire message payload".to_vec();
        let signature = identity.sign(&payload);

        let signed = SignedWireMessage {
            payload: payload.clone(),
            signature: signature.to_bytes(),
            verifying_key: Some(identity.verifying_key_bytes()),
        };

        // Verify with the included key
        let vk = VerifyingKey::from_bytes(&signed.verifying_key.unwrap()).unwrap();
        assert!(verify_signed_message(
            &vk,
            &signed.payload,
            &signed.signature
        ));
    }

    // ── End-to-end: AgentIdentity + WireMessage signing pipeline ───────

    /// Full E2E pipeline: generate identity → encode WireMessage → sign →
    /// verify. This is the exact flow Soong Path uses for authenticated
    /// agent-to-agent communication.
    #[test]
    fn test_e2e_signing_pipeline_with_wire_message() {
        // 1. Two agents generate identities
        let alice = AgentIdentity::generate(AgentId::new("alice"));
        let bob = AgentIdentity::generate(AgentId::new("bob"));

        // 2. Alice creates a WireMessage
        let msg = WireMessage::new(
            MessageType::SigmaProgram,
            AgentId::new("alice"),
            Some(AgentId::new("bob")),
            1,
            b"payload".to_vec(),
        );

        // 3. Alice serializes the message and signs it
        let serialized = crate::tcp_transport::encode_frame(&msg);
        let signature = alice.sign(&serialized);
        let sig_bytes: [u8; 64] = signature.to_bytes();

        // 4. Alice sends a SignedWireMessage with her verifying key
        //    (first contact — key included)
        let signed = SignedWireMessage {
            payload: serialized.clone(),
            signature: sig_bytes,
            verifying_key: Some(alice.verifying_key_bytes()),
        };

        // 5. Bob receives — extracts the verifying key and verifies
        let bob_vk = VerifyingKey::from_bytes(&signed.verifying_key.unwrap()).unwrap();
        assert!(
            verify_signed_message(&bob_vk, &signed.payload, &signed.signature),
            "Bob should trust Alice's signed message"
        );

        // 6. Bob decodes the WireMessage from the verified payload
        let (decoded, _consumed) = crate::tcp_transport::decode_frame(&serialized).unwrap();
        assert_eq!(decoded.sender, AgentId::new("alice"));
        assert_eq!(decoded.recipient, Some(AgentId::new("bob")));
        assert_eq!(decoded.msg_type, MessageType::SigmaProgram);
        assert_eq!(decoded.version, WIRE_VERSION);

        // 7. Tampering is detected — Eve can't forge a message from Alice
        let eve = AgentIdentity::generate(AgentId::new("eve"));
        let forged_payload = crate::tcp_transport::encode_frame(&WireMessage::new(
            MessageType::Heartbeat,
            AgentId::new("alice"), // spoofed sender
            Some(AgentId::new("bob")),
            99,
            b"malicious".to_vec(),
        ));
        let eve_sig = eve.sign(&forged_payload);
        let eve_sig_bytes: [u8; 64] = eve_sig.to_bytes();
        // Bob checks with Alice's key — Eve's signature should fail
        assert!(
            !verify_signed_message(&bob_vk, &forged_payload, &eve_sig_bytes),
            "Eve's signature should NOT verify with Alice's key"
        );
    }

    /// Subsequent messages (after first contact) omit the verifying key.
    /// Bob should already have Alice's key cached and be able to verify.
    #[test]
    fn test_e2e_subsequent_message_without_key() {
        let alice = AgentIdentity::generate(AgentId::new("alice"));

        // Bob has Alice's key from first contact
        let alice_vk = alice.verifying_key.clone();

        // Alice sends a follow-up message (no verifying_key in SignedWireMessage)
        let msg2 = WireMessage::new(
            MessageType::Heartbeat,
            AgentId::new("alice"),
            Some(AgentId::new("bob")),
            2,
            vec![],
        );
        let payload2 = crate::tcp_transport::encode_frame(&msg2);
        let sig2 = alice.sign(&payload2);

        let signed2 = SignedWireMessage {
            payload: payload2.clone(),
            signature: sig2.to_bytes(),
            verifying_key: None, // subsequent message — key already known
        };

        // Bob uses his cached copy of Alice's key
        assert!(verify_signed_message(
            &alice_vk,
            &signed2.payload,
            &signed2.signature
        ));
    }

    /// Multiple agents signing and verifying independently.
    #[test]
    fn test_e2e_multi_agent_signing() {
        let agents: Vec<AgentIdentity> = (0..5)
            .map(|i| AgentIdentity::generate(AgentId::new(format!("agent-{}", i))))
            .collect();

        let mut messages = Vec::new();

        // Each agent signs a message
        for agent in &agents {
            let msg = WireMessage::new(MessageType::Heartbeat, agent.id.clone(), None, 0, vec![]);
            let payload = crate::tcp_transport::encode_frame(&msg);
            let sig = agent.sign(&payload);
            messages.push((agent, payload, sig.to_bytes()));
        }

        // Each agent can verify their own message
        for (agent, payload, sig) in &messages {
            assert!(verify_signed_message(&agent.verifying_key, payload, sig));
        }

        // Cross-agent verification: agent-i's signature fails with agent-j's key
        for i in 0..agents.len() {
            for j in 0..agents.len() {
                if i != j {
                    assert!(
                        !verify_signed_message(
                            &agents[j].verifying_key,
                            &messages[i].1,
                            &messages[i].2
                        ),
                        "agent-{}'s sig should not verify with agent-{}'s key",
                        i,
                        j
                    );
                }
            }
        }
    }
}
