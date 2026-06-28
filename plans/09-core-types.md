# A2X Core Types — a2x-core Crate Design

> **The foundation everything depends on. Zero-dependency primitive types, traits, and enums.**

---

## 1. Overview

`a2x-core` is the **only crate with zero dependencies** (except `std`). Every other crate depends on it. It must be stable, minimal, and well-documented.

**Constraint:** `a2x-core` cannot depend on any other A2X crate. All types referencing other crates (e.g., `VmError` from `a2x-ccs`) must be represented as strings or boxed errors at this layer. Typed error variants belong in the crate that defines them.

- **Crate:** `a2x-core`
- **Dependencies:** None (zero-dependency)
- **Key files:** `lib.rs`, `concept.rs`, `relation.rs`, `graph.rs`, `state.rs`, `policy.rs`, `memory.rs`, `agent.rs`, `packet.rs`, `error.rs`

---

## 2. Core Types — Full Definitions

### ConceptVector

```rust
/// A dense embedding representing a concept, object, event, or abstraction.
/// This is the atomic value type in the A2X language.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConceptVector {
    /// The embedding data.
    pub data: Vec<f32>,
    /// Optional human-readable label (for debug/probe only).
    pub label: Option<String>,
    /// Dimensionality hint (for validation).
    pub dimensions: usize,
}

impl ConceptVector {
    /// Create a zero-initialized vector.
    pub fn zeros(dim: usize) -> Self { /* ... */ }

    /// Create from raw data.
    pub fn from_vec(data: Vec<f32>) -> Self { /* ... */ }

    /// Euclidean norm.
    pub fn norm(&self) -> f32 { /* ... */ }

    /// Cosine similarity with another vector.
    pub fn cosine_similarity(&self, other: &Self) -> f32 { /* ... */ }

    /// Element-wise addition.
    pub fn add(&self, other: &Self) -> Result<Self, CoreError> { /* ... */ }

    /// Element-wise multiplication (Hadamard product).
    pub fn multiply(&self, other: &Self) -> Result<Self, CoreError> { /* ... */ }
}
```

### RelationEdge

```rust
/// A directed, typed edge between two ConceptVectors in the WorldGraph.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RelationEdge {
    /// Source node ID.
    pub source: NodeId,
    /// Target node ID.
    pub target: NodeId,
    /// Type of relation.
    pub relation_type: RelationType,
    /// Learned weight matrix (optional, for neural relations).
    pub weight_matrix: Option<Vec<f32>>,
    /// Strength/confidence of this relation (0.0 – 1.0).
    pub strength: f32,
}
```

### RelationType

```rust
/// Semantic type tag for relations between concepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RelationType {
    /// A causes B (or A is caused by B).
    Causal,
    /// A is located at B (or A contains B spatially).
    Spatial,
    /// A happens before/after B.
    Temporal,
    /// A implies B (or A is a prerequisite for B).
    Logical,
    /// A is a part of B (or A is a type of B).
    Hierarchical,
    /// Custom user-defined relation type (extension mechanism).
    Custom([u8; 4]), // 4-byte namespace ID
}
```

### NodeId

```rust
/// Unique identifier for a node in the WorldGraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(id: u64) -> Self { NodeId(id) }
    pub fn as_u64(&self) -> u64 { self.0 }
}
```

### ActionDistribution

```rust
/// Probability distribution over available actions.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ActionDistribution {
    /// Action labels (for symbolic mapping).
    pub actions: Vec<String>,
    /// Probability for each action.
    pub probabilities: Vec<f32>,
}
```

### AgentId

```rust
/// Unique identifier for an agent or entity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AgentId(String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self { AgentId(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### ProgramId

```rust
/// Content-addressed identifier for a program (Blake3 hash).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProgramId([u8; 32]);

impl ProgramId {
    pub fn new(hash: [u8; 32]) -> Self { ProgramId(hash) }
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
    pub fn compute(data: &[u8]) -> Self {
        // Blake3 hash of content
        let hash = blake3::hash(data);
        ProgramId(*hash.as_bytes())
    }
}
```

---

## 3. Core Traits

### WorldGraph Trait

```rust
/// Trait for the WorldGraph memory (heap).
/// Actual implementation lives in `a2x-ccs`; this trait defines the interface.
pub trait WorldGraph: Clone + Send + Sync {
    type Error: std::error::Error + Send + Sync;

    fn allocate(&mut self, concept: ConceptVector) -> Result<NodeId, Self::Error>;
    fn deallocate(&mut self, id: NodeId) -> Result<(), Self::Error>;
    fn add_edge(&mut self, source: NodeId, target: NodeId, relation: RelationEdge)
        -> Result<(), Self::Error>;
    fn remove_edge(&mut self, source: NodeId, target: NodeId) -> Result<(), Self::Error>;
    fn lookup(&self, id: NodeId) -> Result<Option<&GraphNode>, Self::Error>;
    fn lookup_label(&self, label: &str) -> Result<Option<NodeId>, Self::Error>;
    fn neighbors(&self, id: NodeId) -> Result<Vec<NodeId>, Self::Error>;
    fn query(&self, query: &GraphQuery) -> Result<Vec<NodeId>, Self::Error>;
}
```

### StateField Trait

```rust
/// Trait for the StateField (registers / working memory).
pub trait StateField: Clone + Send + Sync {
    type Error: std::error::Error + Send + Sync;

    fn define_region(&mut self, name: &str, offset: usize, shape: &[usize])
        -> Result<(), Self::Error>;
    fn read_region(&self, name: &str) -> Result<ArrayViewD<f32>, Self::Error>;
    fn write_region(&mut self, name: &str, data: ArrayViewD<f32>)
        -> Result<(), Self::Error>;
    fn snapshot(&self) -> Self;
}
```

### PolicyField Trait

```rust
/// Trait for the policy (JIT compiler + optimizer).
pub trait PolicyField: Send + Sync {
    fn evaluate(&self, state: &dyn StateField, graph: &dyn WorldGraph)
        -> ActionDistribution;
}
```

### MemoryTrace Trait

```rust
/// Trait for execution history.
pub trait MemoryTrace: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    fn push(&mut self, entry: MemoryEntry) -> Result<(), Self::Error>;
    fn tail(&self, n: usize) -> Vec<MemoryEntry>;
    fn compress(&mut self) -> Result<(), Self::Error>;
}
```

### Agent Trait

```rust
/// An execution context for A2X programs.
/// Each agent has a CCS runtime that executes Sigma/Omega programs.
#[async_trait]
pub trait Agent: Send + Sync {
    fn id(&self) -> AgentId;
    fn agent_type(&self) -> AgentType;

    /// Execute a Sigma program on this agent's CCS runtime.
    async fn execute(&self, program: SigmaProgram) -> Result<SigmaProgram, AgentError>;

    /// Execute a compiled Omega program directly (fast path).
    async fn execute_omega(&self, program: OmegaProgram) -> Result<OmegaProgram, AgentError>;

    /// Current internal state (for probing / debug).
    fn state_summary(&self) -> Option<StateSnapshot>;
}
```

### GraphNode & GraphQuery

```rust
/// A node in the WorldGraph.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GraphNode {
    pub id: NodeId,
    pub concept: ConceptVector,
    pub label: Option<String>,
    pub edges: Vec<RelationEdge>,
    pub metadata: NodeMetadata,
}

/// A structured query against the WorldGraph.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GraphQuery {
    ByLabel(String),
    ByRelation(RelationType),
    Neighbors { node: NodeId, max_hops: usize },
    BySimilarity { concept: ConceptVector, threshold: f32 },
    Custom(Vec<u8>),
}

/// Metadata attached to a WorldGraph node.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeMetadata {
    pub access_count: u64,
    pub last_modified: std::time::SystemTime,
    pub ephemeral: bool,
}

/// A single entry in the MemoryTrace (execution history).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MemoryEntry {
    pub timestamp: std::time::SystemTime,
    pub instruction: SigmaPacket,
    pub ip: usize,
    pub state_snapshot: Option<StateField>,
}
```

---

## 4. Core Enums

### AgentType

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AgentType {
    Orchestrator,
    Llm,
    Cli,
    Ccs,
    Omega,
    Entity,
    Custom([u8; 4]),
}
```

### EntityType

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EntityType {
    HumanCli,
    HumanWeb,
    LlmService,
    Application,
    Database,
    Robot,
    CiCd,
    A2xNetwork,
    Custom([u8; 4]),
}
```

### AddressingMode

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressingMode {
    LabelIndex(u32),
    DirectNodeId(NodeId),
    StateFieldRegion(u32),
    Immediate(f32),
}
```

### Opcode

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0x0,
    Bind = 0x1,
    Differentiate = 0x2,
    Ground = 0x3,
    Evolve = 0x4,
    Reflect = 0x5,
    Plan = 0x6,
    Actuate = 0x7,
    Jump = 0x8,
    Branch = 0x9,
    Call = 0xA,
    Return = 0xB,
    Fork = 0xC,
    Merge = 0xD,
    Halt = 0xE,
    Custom(u8),
}
```

### Capability

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    Execute,
    FileSystem,
    Network,
    Shell,
    Probe,
    Custom(String),
}
```

### Modality

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modality {
    Vision,
    Audio,
    Text,
    Proprioception,
    Custom(u8),
}
```

---

## 5. Packet Enum (Unified)

```rust
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Packet {
    Sigma(SigmaPacket),
    Omega(OmegaPacket),
    Raw(Vec<u8>),
}
```

---

## 6. Feature Gating

```toml
# a2x-core/Cargo.toml
[features]
default = ["std"]
std = []
serde = ["dep:serde"]
```

All core types derive `Serialize`/`Deserialize` only when `feature = "serde"` is enabled.

---

## 7. Error Types

### CoreError

```rust
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("dimensionality mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("invalid node ID: {0}")]
    InvalidNodeId(u64),

    #[error("label already exists: {0}")]
    LabelConflict(String),

    #[error("out of memory: cannot allocate node")]
    OutOfMemory,

    #[error("{0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}
```

### AgentError

**Design constraint:** `a2x-core` is zero-dependency. It cannot import `VmError` from `a2x-ccs`. The VM error variant uses a string, not a typed error.

```rust
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("agent {id} not found")]
    NotFound { id: AgentId },

    #[error("agent is at capacity (max {max} concurrent programs)")]
    AtCapacity { max: usize },

    #[error("program {program_id} crashed: {reason}")]
    ProgramCrash { program_id: ProgramId, reason: String },

    #[error("program exceeded time limit of {timeout:?}")]
    Timeout { timeout: std::time::Duration },

    #[error("VM error: {0}")]
    VmError(String),  // string-only at core layer; typed variant lives in a2x-agents

    #[error("transport error: {0}")]
    TransportError(String),

    #[error("safety violation: {0}")]
    SafetyViolation(String),

    #[error("{0}")]
    Core(#[from] CoreError),
}
```

### StateSnapshot

```rust
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StateSnapshot {
    pub agent_id: AgentId,
    pub state: String,
    pub current_program: Option<ProgramId>,
    pub ip: Option<usize>,
    pub world_graph_size: usize,
    pub memory_trace_length: usize,
    pub uptime: std::time::Duration,
}
```

---

## 8. Prelude Module

For convenience, `a2x_core::prelude` re-exports all commonly used types:

```rust
// a2x-core/src/prelude.rs
pub use crate::agent::{Agent, AgentError, AgentId, AgentType};
pub use crate::capability::Capability;
pub use crate::concept::ConceptVector;
pub use crate::entity::EntityType;
pub use crate::error::CoreError;
pub use crate::graph::{GraphNode, GraphQuery, NodeMetadata, WorldGraph};
pub use crate::memory::{MemoryEntry, MemoryTrace};
pub use crate::node::NodeId;
pub use crate::opcode::Opcode;
pub use crate::packet::Packet;
pub use crate::policy::PolicyField;
pub use crate::protocol::ProtocolId;
pub use crate::relation::{RelationEdge, RelationType};
pub use crate::state::{StateField, StateSnapshot};
pub use crate::Modality;
pub use crate::ActionDistribution;
pub use crate::AddressingMode;
```

---

*This sub-plan maps to Phase 0 of the implementation roadmap (first crate to build).*
