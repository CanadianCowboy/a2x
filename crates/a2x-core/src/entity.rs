// See plans/09-core-types.md §2

/// Type tag for external entities connected to the A2X ecosystem.
///
/// Entities are NOT native A2X agents — they connect through the gateway
/// and communicate via protocol adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityType {
    /// Human interacting via CLI terminal.
    HumanCli,
    /// Human interacting via web interface.
    HumanWeb,
    /// External LLM service (e.g., OpenAI API).
    LlmService,
    /// Existing application or microservice.
    Application,
    /// Database or data store.
    Database,
    /// Robot or physical device.
    Robot,
    /// CI/CD pipeline or automation system.
    CiCd,
    /// Another A2X network (federation).
    A2xNetwork,
    /// Custom entity type (4-byte namespace for extension).
    Custom([u8; 4]),
}
