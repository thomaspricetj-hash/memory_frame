use crate::frame::{MemoryFrame, SliceId, CellId};
use std::cmp::Ordering;

/// ---------------------------------------------------------------------------
/// Navigation helpers and public NavTarget enum
/// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTarget {
    NextSlice,
    PrevSlice,
    FirstSlice,
    LastSlice,
}

/// Return slice IDs ordered by a diagonal-style weight based on index.
pub fn slice_ids(frame: &MemoryFrame) -> Vec<SliceId> {
    let mut ids = frame.list_slices();

    ids.sort_by(|a, b| {
        let wa = diagonal_weight_for_slice(frame, a);
        let wb = diagonal_weight_for_slice(frame, b);
        wb.partial_cmp(&wa).unwrap_or(Ordering::Equal)
    });

    ids
}

/// Lookup the slice containing a given cell.
pub fn slice_for_cell(frame: &MemoryFrame, cell: CellId) -> Option<SliceId> {
    frame.find_slice_for_cell(cell)
}

/// Compute diagonal weight based on slice position.
pub fn diagonal_weight_for_slice(frame: &MemoryFrame, slice_id: &SliceId) -> f32 {
    let ids = frame.list_slices();
    let total = ids.len() as f32;

    if total == 0.0 {
        return 0.0;
    }

    let idx = ids.iter().position(|x| x == slice_id).unwrap_or(0) as f32;
    let center = (total - 1.0) / 2.0;
    let dist = (idx - center).abs();

    let w = 1.0 / (1.0 + dist);
    w.clamp(0.25, 1.0)
}
