// See plans/09-core-types.md §2-3

use crate::concept::ConceptVector;
use crate::error::CoreError;
use crate::node::NodeId;
use crate::relation::{RelationEdge, RelationType};

/// A node in the WorldGraph.
///
/// Each node holds a ConceptVector (its semantic content), zero or more
/// outgoing RelationEdges, and metadata for VM bookkeeping.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GraphNode {
    /// Unique node identifier.
    pub id: NodeId,
    /// The concept this node represents.
    pub concept: ConceptVector,
    /// Optional human-readable label (for debug/probe only).
    pub label: Option<String>,
    /// Outgoing edges from this node.
    pub edges: Vec<RelationEdge>,
    /// Bookkeeping metadata.
    pub metadata: NodeMetadata,
}

impl GraphNode {
    /// Create a new GraphNode.
    pub fn new(id: NodeId, concept: ConceptVector) -> Self {
        GraphNode {
            id,
            concept,
            label: None,
            edges: Vec::new(),
            metadata: NodeMetadata::default(),
        }
    }

    /// Create a new GraphNode with a label.
    pub fn with_label(id: NodeId, concept: ConceptVector, label: impl Into<String>) -> Self {
        GraphNode {
            id,
            concept,
            label: Some(label.into()),
            edges: Vec::new(),
            metadata: NodeMetadata::default(),
        }
    }
}

/// Metadata attached to a WorldGraph node for VM bookkeeping.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeMetadata {
    /// Number of times this node has been accessed.
    pub access_count: u64,
    /// When this node was last modified.
    pub last_modified: Option<std::time::SystemTime>,
    /// If true, this node may be evicted under memory pressure.
    pub ephemeral: bool,
    /// Provenance: how this node was created. Populated post-allocation by
    /// `WorldGraph::set_provenance`. Phase 2.B VM dispatch encodes it as
    /// a deterministic string `"<op>(<attrs>)"`, e.g.
    /// `"bind(ip=2,inputs=[a,b])"`. `None` for manually-allocated nodes
    /// (e.g. test fixtures, pre-loaded world-models).
    pub provenance: Option<String>,
}

/// A structured query against the WorldGraph.
///
/// The CCS VM uses queries to find nodes by property, relation, or similarity.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GraphQuery {
    /// Find a node by its label.
    ByLabel(String),
    /// Find nodes connected by a specific relation type.
    ByRelation(RelationType),
    /// Find neighbors of a node within max_hops.
    Neighbors { node: NodeId, max_hops: usize },
    /// Find nodes whose concept is within a similarity threshold.
    BySimilarity {
        concept: ConceptVector,
        threshold: f32,
    },
    /// Custom query (opaque bytes, decoded by the WorldGraph implementation).
    Custom(Vec<u8>),
}

/// Trait for the WorldGraph memory (heap).
///
/// The WorldGraph is the agent's persistent, structured memory. It stores
/// concepts as nodes and relationships as edges. The actual implementation
/// lives in `a2x-ccs` backed by `petgraph`; this trait defines the interface.
pub trait WorldGraph: Send + Sync {
    /// Allocate a new node in the graph (analogous to malloc).
    fn allocate(&mut self, concept: ConceptVector) -> Result<NodeId, CoreError>;

    /// Remove a node and all its edges (analogous to free).
    fn deallocate(&mut self, id: NodeId) -> Result<(), CoreError>;

    /// Add a directed edge between two nodes.
    fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        relation: RelationEdge,
    ) -> Result<(), CoreError>;

    /// Remove an edge between two nodes.
    fn remove_edge(&mut self, source: NodeId, target: NodeId) -> Result<(), CoreError>;

    /// Look up a node by its ID.
    fn lookup(&self, id: NodeId) -> Result<Option<GraphNode>, CoreError>;

    /// Look up a node by its label.
    fn lookup_label(&self, label: &str) -> Result<Option<NodeId>, CoreError>;

    /// Set or change the label of a node. If `label` is already attached to
    /// a different node, returns an error. Setting the same label on the same
    /// node is idempotent (no error).
    fn set_label(&mut self, id: NodeId, label: &str) -> Result<(), CoreError>;

    /// Set or overwrite the provenance string on a node, recording how the
    /// node was created. Does not affect `access_count`, `last_modified`,
    /// or `ephemeral` — those remain managed by the implementation.
    /// Returns `InvalidNodeId` if `id` is not present in the graph.
    /// Idempotent: calling twice with the same value stores it once.
    fn set_provenance(&mut self, id: NodeId, provenance: &str) -> Result<(), CoreError>;

    /// Get the IDs of all neighbors of a node.
    fn neighbors(&self, id: NodeId) -> Result<Vec<NodeId>, CoreError>;

    /// Run a structured query against the graph.
    fn query(&self, query: &GraphQuery) -> Result<Vec<NodeId>, CoreError>;

    /// Number of nodes currently in the graph.
    fn node_count(&self) -> usize;

    /// Number of edges currently in the graph.
    fn edge_count(&self) -> usize;
}
