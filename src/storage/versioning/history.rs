// src/storage/versioning/history.rs
//
// Delta history and basic replay/rollback helpers for cognitive frames.
// This sits on top of DeltaRecord and AnchorFrame and gives you
// a simple, inspectable version log per frame/slice/cell.

use std::collections::VecDeque;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::frame::{SliceId, CellId};
use crate::storage::versioning::delta::{DeltaRecord, DeltaKind, DeltaId};

/// History of deltas for a single frame.
///
/// For now this is an in-memory structure; you can later
/// back it with your storage/folding layer.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeltaHistory {
    pub frame_id: Uuid,
    pub entries: VecDeque<DeltaRecord>,
}

impl DeltaHistory {
    pub fn new(frame_id: Uuid) -> Self {
        Self {
            frame_id,
            entries: VecDeque::new(),
        }
    }

    /// Append a new delta to the history.
    pub fn push(&mut self, delta: DeltaRecord) {
        self.entries.push_back(delta);
    }

    /// List all deltas for a given slice.
    pub fn for_slice(&self, slice_id: &SliceId) -> Vec<&DeltaRecord> {
        self.entries
            .iter()
            .filter(|d| &d.slice_id == slice_id)
            .collect()
    }

    /// List all deltas for a given cell.
    pub fn for_cell(&self, cell_id: &CellId) -> Vec<&DeltaRecord> {
        self.entries
            .iter()
            .filter(|d| d.cell_id.as_ref() == Some(cell_id))
            .collect()
    }

    /// Find a delta by id.
    pub fn get(&self, id: &DeltaId) -> Option<&DeltaRecord> {
        self.entries.iter().find(|d| &d.id == id)
    }

    /// Replay all deltas for a given cell in order.
    pub fn replay_cell(&self, cell_id: &CellId) -> Vec<&DeltaRecord> {
        self.for_cell(cell_id)
    }

    /// Basic rollback: drop all deltas after the given id.
    pub fn rollback_after(&mut self, id: &DeltaId) {
        if let Some(pos) = self.entries.iter().position(|d| &d.id == id) {
            self.entries.truncate(pos + 1);
        }
    }

    /// Detect contradictions for a given cell.
    pub fn contradictions_for_cell(&self, cell_id: &CellId) -> Vec<&DeltaRecord> {
        self.entries
            .iter()
            .filter(|d| d.cell_id.as_ref() == Some(cell_id) && matches!(d.kind, DeltaKind::Contradiction))
            .collect()
    }
}






