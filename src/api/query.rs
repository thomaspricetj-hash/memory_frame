

use crate::frame::{MemoryFrame, Slice, Cell, CellId, SliceId};
use crate::api::ApiError;

/// Query interface for frame-level operations.
pub struct FrameQuery<'a> {
    frame: &'a MemoryFrame,
}

impl<'a> FrameQuery<'a> {
    pub fn new(frame: &'a MemoryFrame) -> Self {
        Self { frame }
    }

    /// Get a slice by ID.
    pub fn get_slice(&self, id: SliceId) -> Result<&Slice, ApiError> {
        self.frame
            .slices
            .get(&id)
            .ok_or(ApiError::SliceNotFound(id.to_string()))
    }

    /// Try-get slice (Option instead of Result).
    pub fn try_slice(&self, id: SliceId) -> Option<&Slice> {
        self.frame.slices.get(&id)
    }

    /// Check if a slice exists.
    pub fn has_slice(&self, id: SliceId) -> bool {
        self.frame.slices.contains_key(&id)
    }

    /// List all slice IDs.
    pub fn list_slices(&self) -> Vec<SliceId> {
        self.frame.slices.keys().cloned().collect()
    }

    /// Create a slice-level query object.
    pub fn slice(&'a self, id: SliceId) -> Result<SliceQuery<'a>, ApiError> {
        Ok(SliceQuery {
            slice: self.get_slice(id)?,
        })
    }

    /// Create a cell-level query object.
    pub fn cell(&'a self) -> CellQuery<'a> {
        CellQuery { frame: self.frame }
    }
}

/// Query interface for slice-level operations.
pub struct SliceQuery<'a> {
    slice: &'a Slice,
}

impl<'a> SliceQuery<'a> {
    /// Get a cell by ID.
    pub fn get_cell(&self, id: CellId) -> Result<&Cell, ApiError> {
        self.slice
            .grid
            .get_cell(id)
            .ok_or(ApiError::CellNotFound(id.to_string()))
    }

    /// Try-get cell (Option instead of Result).
    pub fn try_cell(&self, id: CellId) -> Option<&Cell> {
        self.slice.grid.get_cell(id)
    }

    /// Get neighbors of a cell.
    pub fn neighbors(&self, id: CellId) -> Vec<CellId> {
        self.slice.grid.neighbors(id)
    }

    /// List all cell IDs.
    pub fn list_cells(&self) -> Vec<CellId> {
        self.slice.grid.cells.iter().map(|c| c.id).collect()
    }

    /// Access underlying slice.
    pub fn raw(&self) -> &'a Slice {
        self.slice
    }
}

/// Query interface for cross-slice cell operations.
pub struct CellQuery<'a> {
    frame: &'a MemoryFrame,
}

impl<'a> CellQuery<'a> {
    /// Get all cross-layer links from a cell.
    pub fn cross_links(&self, id: CellId) -> Vec<CellId> {
        self.frame.cross_connect.links_from(id)
    }

    /// Check if a cell exists in any slice.
    pub fn exists(&self, id: CellId) -> bool {
        self.frame
            .slices
            .values()
            .any(|slice| slice.grid.get_cell(id).is_some())
    }

    /// Find the slice that contains a given cell.
    pub fn find_slice(&self, id: CellId) -> Option<SliceId> {
        self.frame
            .slices
            .iter()
            .find(|(_, slice)| slice.grid.get_cell(id).is_some())
            .map(|(sid, _)| sid.clone())   // FIXED: LayerId/SliceId is not Copy
    }
}






