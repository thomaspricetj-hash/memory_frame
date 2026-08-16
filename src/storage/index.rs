// src/storage/index.rs
//
// Highâ€‘performance lookup tables for slices, cells, and graph nodes.
// This indexing layer accelerates:
// - cell lookup
// - slice lookup
// - graph node lookup
// - reverse mappings
//
// It is intentionally lightweight and inâ€‘memory.
// Later you can back it with your folding/storage layer.

use std::collections::{HashMap, HashSet};

use crate::frame::{SliceId, CellId};
use crate::symbolic::graph::GraphNodeId;

/// Global index for a CognitiveFrame.
///
/// This structure is rebuilt or updated as slices/cells change.
/// It is NOT the source of truth â€” slices and cells are.
/// The index simply accelerates access.
#[derive(Debug, Default)]
pub struct FrameIndex {
    /// Map: SliceId â†’ all CellIds in that slice.
    pub slice_cells: HashMap<SliceId, HashSet<CellId>>,

    /// Map: CellId â†’ SliceId (reverse lookup).
    pub cell_to_slice: HashMap<CellId, SliceId>,

    /// Map: GraphNodeId â†’ label (for fast symbolic lookup).
    pub graph_labels: HashMap<GraphNodeId, String>,
}

impl FrameIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cell inside a slice.
    pub fn add_cell(&mut self, slice_id: SliceId, cell_id: CellId) {
        self.slice_cells
            .entry(slice_id.clone())
            .or_default()
            .insert(cell_id.clone());

        self.cell_to_slice.insert(cell_id, slice_id);
    }

    /// Remove a cell from the index.
    pub fn remove_cell(&mut self, cell_id: &CellId) {
        if let Some(slice_id) = self.cell_to_slice.remove(cell_id) {
            if let Some(set) = self.slice_cells.get_mut(&slice_id) {
                set.remove(cell_id);
            }
        }
    }

    /// Get all cells in a slice.
    pub fn cells_in_slice(&self, slice_id: &SliceId) -> Option<&HashSet<CellId>> {
        self.slice_cells.get(slice_id)
    }

    /// Find which slice a cell belongs to.
    pub fn slice_of_cell(&self, cell_id: &CellId) -> Option<&SliceId> {
        self.cell_to_slice.get(cell_id)
    }

    /// Register a graph node label for fast lookup.
    pub fn add_graph_label(&mut self, node_id: GraphNodeId, label: impl Into<String>) {
        self.graph_labels.insert(node_id, label.into());
    }

    /// Lookup a graph node label.
    pub fn graph_label(&self, node_id: &GraphNodeId) -> Option<&String> {
        self.graph_labels.get(node_id)
    }
}






