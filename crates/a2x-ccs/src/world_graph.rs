// See plans/03-ccs-vm.md §5

use std::collections::HashMap;

use a2x_core::concept::ConceptVector;
use a2x_core::error::CoreError;
use a2x_core::graph::{GraphNode, GraphQuery, NodeMetadata, WorldGraph};
use a2x_core::node::NodeId;
use a2x_core::relation::RelationEdge;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use petgraph::Direction;

/// Internal node weight stored in petgraph.
#[derive(Clone, Debug)]
struct NodePayload {
    id: NodeId,
    concept: ConceptVector,
    label: Option<String>,
    edges: Vec<RelationEdge>,
    metadata: NodeMetadata,
}

/// Default maximum nodes in the WorldGraph.
const DEFAULT_MAX_NODES: usize = 1_000_000;

/// petgraph-backed implementation of the WorldGraph trait.
///
/// Uses `petgraph::StableGraph` so that NodeIds (u64) remain stable even after
/// deletions. Maintains label → NodeId and incoming-edge indices for efficient
/// lookups.
pub struct PetgraphWorldGraph {
    /// The underlying graph store.
    graph: StableGraph<NodePayload, RelationEdge>,
    /// Label → NodeId index for O(1) lookup.
    label_index: HashMap<String, NodeId>,
    /// NodeId → list of incoming edge info (source NodeId, edge).
    incoming: HashMap<NodeId, Vec<(NodeId, RelationEdge)>>,
    /// Next NodeId to allocate (monotonically increasing).
    next_id: u64,
    /// Maximum number of nodes allowed.
    max_nodes: usize,
    /// Maps NodeId to petgraph NodeIndex for fast graph ops.
    id_to_index: HashMap<NodeId, NodeIndex>,
}

impl PetgraphWorldGraph {
    /// Create a new empty WorldGraph.
    pub fn new() -> Self {
        PetgraphWorldGraph {
            graph: StableGraph::new(),
            label_index: HashMap::new(),
            incoming: HashMap::new(),
            next_id: 1,
            max_nodes: DEFAULT_MAX_NODES,
            id_to_index: HashMap::new(),
        }
    }

    /// Create with a custom node capacity limit.
    pub fn with_capacity(max_nodes: usize) -> Self {
        PetgraphWorldGraph {
            max_nodes,
            ..Self::new()
        }
    }

    /// Get petgraph NodeIndex for a NodeId.
    fn node_index(&self, id: NodeId) -> Option<NodeIndex> {
        self.id_to_index.get(&id).copied()
    }

    /// Get all incoming edges for a node.
    pub fn incoming_edges(&self, id: NodeId) -> Vec<&RelationEdge> {
        self.incoming
            .get(&id)
            .map(|v| v.iter().map(|(_, e)| e).collect())
            .unwrap_or_default()
    }

    /// Get all outgoing edges for a node.
    pub fn outgoing_edges(&self, id: NodeId) -> Vec<&RelationEdge> {
        match self.node_index(id) {
            Some(ni) => self
                .graph
                .edges_directed(ni, Direction::Outgoing)
                .map(|e| e.weight())
                .collect(),
            None => Vec::new(),
        }
    }
}

impl Default for PetgraphWorldGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldGraph for PetgraphWorldGraph {
    fn allocate(&mut self, concept: ConceptVector) -> Result<NodeId, CoreError> {
        if self.graph.node_count() >= self.max_nodes {
            return Err(CoreError::Other(
                format!(
                    "graph at capacity: {}/{} nodes",
                    self.graph.node_count(),
                    self.max_nodes
                )
                .into(),
            ));
        }

        let id = NodeId::new(self.next_id);
        self.next_id += 1;

        let payload = NodePayload {
            id,
            concept,
            label: None,
            edges: Vec::new(),
            metadata: NodeMetadata::default(),
        };

        let ni = self.graph.add_node(payload);
        self.id_to_index.insert(id, ni);

        Ok(id)
    }

    fn deallocate(&mut self, id: NodeId) -> Result<(), CoreError> {
        let ni = self
            .node_index(id)
            .ok_or(CoreError::InvalidNodeId(id.as_u64()))?;

        // Remove all outgoing edge references from incoming index
        let outgoing_edges: Vec<RelationEdge> = self
            .graph
            .edges_directed(ni, Direction::Outgoing)
            .map(|e| e.weight().clone())
            .collect();
        for edge in &outgoing_edges {
            if let Some(in_list) = self.incoming.get_mut(&edge.target) {
                in_list.retain(|(src, _)| *src != id);
            }
        }

        // Remove all incoming edge references
        if let Some(in_list) = self.incoming.remove(&id) {
            for (src_id, _) in &in_list {
                if let Some(src_ni) = self.node_index(*src_id) {
                    // Remove edges from source to this node
                    let mut to_remove = Vec::new();
                    for edge_ref in self.graph.edges_directed(src_ni, Direction::Outgoing) {
                        if edge_ref.target() == ni {
                            to_remove.push(edge_ref.id());
                        }
                    }
                    for eid in to_remove {
                        self.graph.remove_edge(eid);
                    }
                }
            }
        }

        // Remove from label index
        if let Some(payload) = self.graph.node_weight(ni) {
            if let Some(ref label) = payload.label {
                self.label_index.remove(label);
            }
        }

        // Remove from id index and graph
        self.id_to_index.remove(&id);
        self.graph.remove_node(ni);

        Ok(())
    }

    fn add_edge(
        &mut self,
        source: NodeId,
        target: NodeId,
        relation: RelationEdge,
    ) -> Result<(), CoreError> {
        if source == target {
            return Err(CoreError::Other(
                format!("self-loop not allowed on node {}", source.as_u64()).into(),
            ));
        }

        let src_ni = self
            .node_index(source)
            .ok_or(CoreError::InvalidNodeId(source.as_u64()))?;
        let tgt_ni = self
            .node_index(target)
            .ok_or(CoreError::InvalidNodeId(target.as_u64()))?;

        // Check for duplicate edge
        for edge_ref in self.graph.edges_directed(src_ni, Direction::Outgoing) {
            if edge_ref.target() == tgt_ni
                && edge_ref.weight().relation_type == relation.relation_type
            {
                return Err(CoreError::Other(
                    format!(
                        "edge {} -> {} with type {:?} already exists",
                        source.as_u64(),
                        target.as_u64(),
                        relation.relation_type
                    )
                    .into(),
                ));
            }
        }

        self.graph.add_edge(src_ni, tgt_ni, relation.clone());

        // Update source node's edge list
        if let Some(payload) = self.graph.node_weight_mut(src_ni) {
            payload.edges.push(relation.clone());
        }

        // Update incoming index
        self.incoming
            .entry(target)
            .or_default()
            .push((source, relation));

        Ok(())
    }

    fn remove_edge(&mut self, source: NodeId, target: NodeId) -> Result<(), CoreError> {
        let src_ni = self
            .node_index(source)
            .ok_or(CoreError::InvalidNodeId(source.as_u64()))?;
        let tgt_ni = self
            .node_index(target)
            .ok_or(CoreError::InvalidNodeId(target.as_u64()))?;

        let mut edge_to_remove = None;
        for edge_ref in self.graph.edges_directed(src_ni, Direction::Outgoing) {
            if edge_ref.target() == tgt_ni {
                edge_to_remove = Some(edge_ref.id());
                break;
            }
        }

        match edge_to_remove {
            Some(eid) => {
                self.graph.remove_edge(eid);

                // Update source node's edge list
                if let Some(payload) = self.graph.node_weight_mut(src_ni) {
                    payload.edges.retain(|e| e.target != target);
                }

                // Update incoming index
                if let Some(in_list) = self.incoming.get_mut(&target) {
                    in_list.retain(|(src, _)| *src != source);
                }

                Ok(())
            }
            None => Err(CoreError::Other(
                format!("edge {} -> {} not found", source.as_u64(), target.as_u64()).into(),
            )),
        }
    }

    fn lookup(&self, id: NodeId) -> Result<Option<GraphNode>, CoreError> {
        match self.node_index(id) {
            Some(ni) => {
                let payload = self.graph.node_weight(ni).unwrap();
                Ok(Some(GraphNode {
                    id: payload.id,
                    concept: payload.concept.clone(),
                    label: payload.label.clone(),
                    edges: payload.edges.clone(),
                    metadata: payload.metadata.clone(),
                }))
            }
            None => Ok(None),
        }
    }

    fn lookup_label(&self, label: &str) -> Result<Option<NodeId>, CoreError> {
        Ok(self.label_index.get(label).copied())
    }

    fn set_label(&mut self, id: NodeId, label: &str) -> Result<(), CoreError> {
        let ni = self
            .node_index(id)
            .ok_or(CoreError::InvalidNodeId(id.as_u64()))?;

        // If a different node already holds this label, conflict.
        if let Some(existing) = self.label_index.get(label) {
            if *existing != id {
                return Err(CoreError::Other(
                    format!(
                        "label '{}' is already attached to node {}",
                        label,
                        existing.as_u64()
                    )
                    .into(),
                ));
            }
            // Same id, same label — idempotent.
            return Ok(());
        }

        // Drop any previous label this node carried (it's leaving the index).
        if let Some(old) = self.graph.node_weight(ni).and_then(|p| p.label.clone()) {
            self.label_index.remove(&old);
        }

        if let Some(payload) = self.graph.node_weight_mut(ni) {
            payload.label = Some(label.to_string());
        }
        self.label_index.insert(label.to_string(), id);

        Ok(())
    }

    fn neighbors(&self, id: NodeId) -> Result<Vec<NodeId>, CoreError> {
        let ni = self
            .node_index(id)
            .ok_or(CoreError::InvalidNodeId(id.as_u64()))?;
        let neighbors: Vec<NodeId> = self
            .graph
            .edges_directed(ni, Direction::Outgoing)
            .map(|e| e.weight().target)
            .collect();
        Ok(neighbors)
    }

    fn query(&self, query: &GraphQuery) -> Result<Vec<NodeId>, CoreError> {
        let results: Vec<NodeId> = match query {
            GraphQuery::ByLabel(label) => {
                self.label_index.get(label).into_iter().copied().collect()
            }
            GraphQuery::ByRelation(rel_type) => self
                .graph
                .edge_references()
                .filter(|e| e.weight().relation_type == *rel_type)
                .map(|e| e.weight().target)
                .collect(),
            GraphQuery::Neighbors { node, max_hops } => {
                let mut visited: Vec<NodeId> = Vec::new();
                let mut frontier: Vec<NodeId> = vec![*node];
                for _ in 0..*max_hops {
                    let mut next_frontier = Vec::new();
                    for nid in &frontier {
                        if let Some(ni) = self.node_index(*nid) {
                            for edge_ref in self.graph.edges_directed(ni, Direction::Outgoing) {
                                let tgt = edge_ref.weight().target;
                                if !visited.contains(&tgt) && !frontier.contains(&tgt) {
                                    next_frontier.push(tgt);
                                }
                            }
                        }
                    }
                    visited.extend(frontier);
                    frontier = next_frontier;
                    if frontier.is_empty() {
                        break;
                    }
                }
                visited.extend(frontier);
                visited.retain(|n| *n != *node);
                visited
            }
            GraphQuery::BySimilarity { concept, threshold } => self
                .graph
                .node_weights()
                .filter(|n| n.concept.cosine_similarity(concept) >= *threshold)
                .map(|n| n.id)
                .collect(),
            GraphQuery::Custom(_) => Vec::new(), // Phase 0 stub
        };
        Ok(results)
    }

    fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a2x_core::relation::RelationType;

    fn make_concept(vals: Vec<f32>) -> ConceptVector {
        ConceptVector::from_vec(vals)
    }

    #[test]
    fn test_allocate_and_lookup() {
        let mut wg = PetgraphWorldGraph::new();
        let c = make_concept(vec![1.0, 2.0]);
        let id = wg.allocate(c.clone()).unwrap();
        let node = wg.lookup(id).unwrap().unwrap();
        assert_eq!(node.concept, c);
        assert_eq!(wg.node_count(), 1);
    }

    #[test]
    fn test_deallocate() {
        let mut wg = PetgraphWorldGraph::new();
        let id = wg.allocate(make_concept(vec![1.0])).unwrap();
        assert_eq!(wg.node_count(), 1);
        wg.deallocate(id).unwrap();
        assert_eq!(wg.node_count(), 0);
        assert!(wg.lookup(id).unwrap().is_none());
    }

    #[test]
    fn test_add_edge() {
        let mut wg = PetgraphWorldGraph::new();
        let a = wg.allocate(make_concept(vec![1.0])).unwrap();
        let b = wg.allocate(make_concept(vec![2.0])).unwrap();
        let edge = RelationEdge::new(a, b, RelationType::Causal, 0.9);
        wg.add_edge(a, b, edge).unwrap();
        assert_eq!(wg.edge_count(), 1);
        let neighbors = wg.neighbors(a).unwrap();
        assert_eq!(neighbors, vec![b]);
    }

    #[test]
    fn test_self_loop_rejected() {
        let mut wg = PetgraphWorldGraph::new();
        let a = wg.allocate(make_concept(vec![1.0])).unwrap();
        let edge = RelationEdge::new(a, a, RelationType::Causal, 0.5);
        assert!(wg.add_edge(a, a, edge).is_err());
    }

    #[test]
    fn test_query_by_similarity() {
        let mut wg = PetgraphWorldGraph::new();
        let c1 = make_concept(vec![1.0, 0.0, 0.0]);
        let c2 = make_concept(vec![0.0, 1.0, 0.0]);
        let c3 = make_concept(vec![0.0, 0.0, 1.0]);
        wg.allocate(c1.clone()).unwrap();
        wg.allocate(c2.clone()).unwrap();
        wg.allocate(c3.clone()).unwrap();
        let results = wg
            .query(&GraphQuery::BySimilarity {
                concept: c1,
                threshold: 0.9,
            })
            .unwrap();
        assert_eq!(results.len(), 1); // only c1 matches itself
    }

    #[test]
    fn test_at_capacity() {
        let mut wg = PetgraphWorldGraph::with_capacity(2);
        wg.allocate(make_concept(vec![1.0])).unwrap();
        wg.allocate(make_concept(vec![2.0])).unwrap();
        assert!(wg.allocate(make_concept(vec![3.0])).is_err());
    }

    #[test]
    fn test_set_label() {
        let mut wg = PetgraphWorldGraph::new();
        let id = wg.allocate(make_concept(vec![1.0])).unwrap();
        wg.set_label(id, "sys").unwrap();
        assert_eq!(wg.lookup_label("sys").unwrap(), Some(id));
        let node = wg.lookup(id).unwrap().unwrap();
        assert_eq!(node.label.as_deref(), Some("sys"));
    }

    #[test]
    fn test_set_label_conflict_on_different_node() {
        let mut wg = PetgraphWorldGraph::new();
        let a = wg.allocate(make_concept(vec![1.0])).unwrap();
        let b = wg.allocate(make_concept(vec![2.0])).unwrap();
        wg.set_label(a, "dup").unwrap();
        assert!(wg.set_label(b, "dup").is_err());
    }

    #[test]
    fn test_set_label_replaces_old_label() {
        let mut wg = PetgraphWorldGraph::new();
        let id = wg.allocate(make_concept(vec![1.0])).unwrap();
        wg.set_label(id, "first").unwrap();
        wg.set_label(id, "second").unwrap();
        assert_eq!(wg.lookup_label("first").unwrap(), None);
        assert_eq!(wg.lookup_label("second").unwrap(), Some(id));
    }

    #[test]
    fn test_set_label_idempotent() {
        let mut wg = PetgraphWorldGraph::new();
        let id = wg.allocate(make_concept(vec![1.0])).unwrap();
        wg.set_label(id, "x").unwrap();
        wg.set_label(id, "x").unwrap(); // idempotent
        assert_eq!(wg.lookup_label("x").unwrap(), Some(id));
    }
}
