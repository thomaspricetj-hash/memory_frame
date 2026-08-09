use crate::frame::{MemoryFrame, SliceId, CellId};
use std::cmp::Ordering;

/// Return slice IDs ordered by a simple diagonal-style weight based on index.
/// This keeps the helper lightweight and fully compatible with `MemoryFrame`.
pub fn slice_ids(frame: &MemoryFrame) -> Vec<SliceId> {
    let mut ids = frame.list_slices();

    ids.sort_by(|a, b| {
        let wa = diagonal_weight_for_slice(frame, a);
        let wb = diagonal_weight_for_slice(frame, b);
        wb.partial_cmp(&wa).unwrap_or(Ordering::Equal)
    });

    ids
}

/// Diagonal-aware slice lookup for a cell, but still using the existing
/// `find_slice_for_cell` API so we don't depend on non-existent methods.
pub fn slice_for_cell(frame: &MemoryFrame, cell: CellId) -> Option<SliceId> {
    // Fast path: use the engine's own mapping.
    frame.find_slice_for_cell(cell)
}

/// Compute a simple diagonal weight for a slice based on its position
/// in the slice list. This is safe and uses only existing APIs.
fn diagonal_weight_for_slice(frame: &MemoryFrame, slice_id: &SliceId) -> f32 {
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
