// See plans/09-core-types.md §2

/// Capability tags for agents and entities.
///
/// Each agent advertises its capabilities through the bus discovery protocol.
/// The router matches programs to agents based on required capabilities.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Capability {
    /// Execute arbitrary Σ∞ programs.
    Execute,
    /// Read/write the file system.
    FileSystem,
    /// Make network requests.
    Network,
    /// Execute shell commands.
    Shell,
    /// Probe/inspect agent state.
    Probe,
    /// Chat/conversational interaction.
    Chat,
    /// Generate Σ∞ programs from natural language.
    Generate,
    /// Reflect/self-model cognitive state.
    Reflect,
    /// Custom capability (free-form string).
    Custom(String),
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::Execute => write!(f, "execute"),
            Capability::FileSystem => write!(f, "fs"),
            Capability::Network => write!(f, "net"),
            Capability::Shell => write!(f, "shell"),
            Capability::Probe => write!(f, "probe"),
            Capability::Chat => write!(f, "chat"),
            Capability::Generate => write!(f, "generate"),
            Capability::Reflect => write!(f, "reflect"),
            Capability::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}
