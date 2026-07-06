// See plans/04-bus.md §4

use a2x_core::{AgentId, ProgramId};

/// Wire protocol version.
pub const WIRE_VERSION: u8 = 0x01;

/// A message on the wire — the unified format for all bus communication.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WireMessage {
    /// Protocol version (currently 0x01).
    pub version: u8,
    /// Message type.
    pub msg_type: MessageType,
    /// Sender agent ID.
    pub sender: AgentId,
    /// Recipient agent ID (broadcast if None).
    pub recipient: Option<AgentId>,
    /// Correlation ID for request-response matching.
    pub correlation_id: u64,
    /// Timestamp (milliseconds since epoch).
    pub timestamp: u64,
    /// Raw payload bytes.
    pub payload: Vec<u8>,
}

impl WireMessage {
    /// Create a new message with the current timestamp.
    pub fn new(
        msg_type: MessageType,
        sender: AgentId,
        recipient: Option<AgentId>,
        correlation_id: u64,
        payload: Vec<u8>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        WireMessage {
            version: WIRE_VERSION,
            msg_type,
            sender,
            recipient,
            correlation_id,
            timestamp,
            payload,
        }
    }
}

/// Types of messages that can be sent over the wire.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MessageType {
    /// A Σ∞ program to execute (carried as serialized bytes).
    SigmaProgram,
    /// A compiled Ω program to execute (carried as serialized bytes).
    OmegaProgram,
    /// Agent discovery / announcement.
    Announce,
    /// Request for a cached program by ID.
    ProgramRequest(ProgramId),
    /// Response with a cached program.
    ProgramResponse {
        id: ProgramId,
        program_bytes: Vec<u8>,
    },
    /// Error response.
    Error(WireError),
    /// Heartbeat / keepalive.
    Heartbeat,
}

impl MessageType {
    /// Stable string representation for routing/dispatch (not Debug).
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageType::SigmaProgram => "sigma_program",
            MessageType::OmegaProgram => "omega_program",
            MessageType::Announce => "announce",
            MessageType::ProgramRequest(_) => "program_request",
            MessageType::ProgramResponse { .. } => "program_response",
            MessageType::Error(_) => "error",
            MessageType::Heartbeat => "heartbeat",
        }
    }
}

/// Simple wire-level error.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WireError {
    pub code: u16,
    pub message: String,
}
