// See plans/09-core-types.md §8

//! Re-exports of all commonly used types from `a2x-core`.
//!
//! Importing `a2x_core::prelude::*` gives you everything you typically need
//! when building on top of the A2X foundation.

pub use crate::addressing::AddressingMode;
pub use crate::agent::Agent;
pub use crate::agent_id::{AgentId, AgentType};
pub use crate::capability::Capability;
pub use crate::concept::ConceptVector;
pub use crate::entity::EntityType;
pub use crate::error::{AgentError, CoreError};
pub use crate::graph::{GraphNode, GraphQuery, NodeMetadata, WorldGraph};
pub use crate::memory::{MemoryEntry, MemoryTrace};
pub use crate::modality::Modality;
pub use crate::node::NodeId;
pub use crate::opcode::Opcode;
pub use crate::packet::Packet;
pub use crate::policy::{ActionDistribution, PolicyField};
pub use crate::program_id::ProgramId;
pub use crate::protocol::ProtocolId;
pub use crate::relation::{RelationEdge, RelationType};
pub use crate::state::{StateField, StateSnapshot};
