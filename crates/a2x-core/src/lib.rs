// a2x-core — Zero-dependency primitive types, traits, and enums for the A2X ecosystem.
//
// See plans/09-core-types.md for the full design specification.

// Modules
pub mod addressing;
pub mod agent;
pub mod agent_id;
pub mod capability;
pub mod concept;
pub mod entity;
pub mod error;
pub mod graph;
pub mod memory;
pub mod modality;
pub mod node;
pub mod opcode;
pub mod packet;
pub mod policy;
pub mod program_id;
pub mod protocol;
pub mod relation;
pub mod state;

// Convenience re-exports
pub mod prelude;

// Re-export commonly-used types at crate root for ergonomic access
pub use addressing::AddressingMode;
pub use agent::Agent;
pub use agent_id::{AgentId, AgentType};
pub use capability::Capability;
pub use concept::ConceptVector;
pub use entity::EntityType;
pub use error::{AgentError, CoreError};
pub use graph::{GraphNode, GraphQuery, NodeMetadata, WorldGraph};
pub use memory::{MemoryEntry, MemoryTrace};
pub use modality::Modality;
pub use node::NodeId;
pub use opcode::Opcode;
pub use packet::Packet;
pub use policy::{ActionDistribution, PolicyField};
pub use program_id::ProgramId;
pub use protocol::ProtocolId;
pub use relation::{RelationEdge, RelationType};
pub use state::{StateField, StateSnapshot};
