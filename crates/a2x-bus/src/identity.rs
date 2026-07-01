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
        let message = "⟦Σ∞⟧⟬I:✦ ∷ C:⟨test⟩⟭".as_bytes();

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
}
