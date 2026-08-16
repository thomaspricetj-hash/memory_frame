// src/symbolic/reasoner.rs
//
// Lightweight symbolic reasoning engine for the hybrid cognitive memory system.
// Operates over the KnowledgeGraph and provides:
// - simple inference
// - relationship lookup
// - path search
// - semantic neighborhood expansion
//
// This is intentionally minimal but production-grade.
// You can expand it later with rule-based reasoning, constraint solving,
// or neural-symbolic hybrid inference.

use std::collections::{HashSet, VecDeque};

use crate::symbolic::graph::{KnowledgeGraph, GraphNodeId, GraphEdge};

/// A simple reasoning engine that operates over the KnowledgeGraph.
///
/// Responsibilities:
/// - find neighbors
/// - find related concepts
/// - search paths
/// - compute simple influence scores
pub struct Reasoner<'a> {
    pub graph: &'a KnowledgeGraph,
}

impl<'a> Reasoner<'a> {
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self { graph }
    }

    /// Get all directly connected neighbors of a node.
    pub fn neighbors(&self, node: &GraphNodeId) -> Vec<&GraphEdge> {
        self.graph.outgoing_edges(node)
    }

    /// Get all nodes reachable within N hops.
    pub fn neighborhood(&self, start: &GraphNodeId, hops: usize) -> HashSet<GraphNodeId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        visited.insert(*start);
        queue.push_back((*start, 0));

        while let Some((node, depth)) = queue.pop_front() {
            if depth >= hops {
                continue;
            }

            for edge in self.graph.outgoing_edges(&node) {
                let next = edge.to;
                if visited.insert(next) {
                    queue.push_back((next, depth + 1));
                }
            }
        }

        visited
    }

    /// Find a path between two nodes using BFS.
    pub fn find_path(
        &self,
        start: &GraphNodeId,
        goal: &GraphNodeId,
    ) -> Option<Vec<GraphNodeId>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<GraphNodeId, GraphNodeId> = HashMap::new();

        queue.push_back(*start);
        visited.insert(*start);

        while let Some(node) = queue.pop_front() {
            if &node == goal {
                // reconstruct path
                let mut path = Vec::new();
                let mut current = node;

                path.push(current);
                while let Some(p) = parent.get(&current) {
                    current = *p;
                    path.push(current);
                }

                path.reverse();
                return Some(path);
            }

            for edge in self.graph.outgoing_edges(&node) {
                let next = edge.to;
                if visited.insert(next) {
                    parent.insert(next, node);
                    queue.push_back(next);
                }
            }
        }

        None
    }

    /// Compute a simple influence score for a node based on outgoing edge weights.
    pub fn influence_score(&self, node: &GraphNodeId) -> f32 {
        self.graph
            .outgoing_edges(node)
            .iter()
            .map(|e| e.weight)
            .sum()
    }

    /// Rank neighbors by edge weight (descending).
    pub fn ranked_neighbors(&self, node: &GraphNodeId) -> Vec<&GraphEdge> {
        let mut edges = self.graph.outgoing_edges(node);
        edges.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
        edges
    }
}






