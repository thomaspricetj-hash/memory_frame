// src/symbolic/graph.rs
//
// Lightweight symbolic knowledge graph for the hybrid cognitive engine.
// This module provides nodes, edges, and a simple graph structure
// that other layers (CrossRef, Semantic, Rule, Gesture) can use.
//
// This is intentionally minimal but production-grade:
// - fast inserts
// - fast lookups
// - stable identifiers
// - no external dependencies beyond uuid + std

use std::collections::{HashMap, HashSet};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Unique identifier for a graph node.
pub type GraphNodeId = Uuid;

/// Unique identifier for a graph edge.
pub type GraphEdgeId = Uuid;

/// A node in the symbolic knowledge graph.
///
/// Nodes represent concepts, entities, gestures, topics, or anything
/// that can be referenced symbolically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: GraphNodeId,
    pub label: String,
    pub tags: Vec<String>,
}

/// A directed edge between two nodes.
///
/// Edges represent relationships:
/// - semantic links
/// - gesture â†’ concept mappings
/// - topic â†’ subtopic
/// - entity â†’ attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: GraphEdgeId,
    pub from: GraphNodeId,
    pub to: GraphNodeId,
    pub relation: String,
    pub weight: f32,
}

/// The symbolic knowledge graph.
///
/// This is a lightweight, in-memory graph optimized for:
/// - fast inserts
/// - fast lookups
/// - simple traversal
///
/// You can later back this with your storage/folding layer.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<GraphNodeId, GraphNode>,
    pub edges: HashMap<GraphEdgeId, GraphEdge>,

    /// Reverse adjacency for fast traversal.
    pub outgoing: HashMap<GraphNodeId, HashSet<GraphEdgeId>>,
    pub incoming: HashMap<GraphNodeId, HashSet<GraphEdgeId>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new node to the graph.
    pub fn add_node(&mut self, label: impl Into<String>, tags: Vec<String>) -> GraphNodeId {
        let id = GraphNodeId::new_v4();
        let node = GraphNode {
            id,
            label: label.into(),
            tags,
        };
        self.nodes.insert(id, node);
        id
    }

    /// Add a directed edge between two nodes.
    pub fn add_edge(
        &mut self,
        from: GraphNodeId,
        to: GraphNodeId,
        relation: impl Into<String>,
        weight: f32,
    ) -> GraphEdgeId {
        let id = GraphEdgeId::new_v4();
        let edge = GraphEdge {
            id,
            from,
            to,
            relation: relation.into(),
            weight,
        };

        self.edges.insert(id, edge);

        self.outgoing.entry(from).or_default().insert(id);
        self.incoming.entry(to).or_default().insert(id);

        id
    }

    /// Get a node by id.
    pub fn node(&self, id: &GraphNodeId) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Get an edge by id.
    pub fn edge(&self, id: &GraphEdgeId) -> Option<&GraphEdge> {
        self.edges.get(id)
    }

    /// Get all outgoing edges from a node.
    pub fn outgoing_edges(&self, id: &GraphNodeId) -> Vec<&GraphEdge> {
        self.outgoing
            .get(id)
            .into_iter()
            .flat_map(|set| set.iter())
            .filter_map(|eid| self.edges.get(eid))
            .collect()
    }

    /// Get all incoming edges to a node.
    pub fn incoming_edges(&self, id: &GraphNodeId) -> Vec<&GraphEdge> {
        self.incoming
            .get(id)
            .into_iter()
            .flat_map(|set| set.iter())
            .filter_map(|eid| self.edges.get(eid))
            .collect()
    }
}






