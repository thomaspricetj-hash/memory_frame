// src/frame/cognitive_frame.rs
//
// Top-level hybrid cognitive frame:
// - Owns all slices (core, delta, fact-check, cross-ref, semantic, emotional, rule, sign-language, short-term).
// - Provides a stable container for multi-modal memory.
// - Orchestrates navigation, ingestion, and high-level passes (fact-check, cross-ref, rules).
//
// This is intentionally conservative: no external dependencies beyond what your crate
// already uses (uuid, chrono, std collections).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::frame::{Slice, SliceId, Cell, CellId};

/// Stable identifier for a cognitive frame.
/// For now this is just a UUID; you can later swap to a stronger type if needed.
pub type FrameId = Uuid;

/// Lightweight metadata for a cognitive frame.
/// Extend this as needed (tags, owner, modality, etc.).
#[derive(Debug, Clone, Default)]
pub struct FrameMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// Top-level hybrid cognitive frame.
///
/// This sits above your existing MemoryFrame / Slice / Cell structures and
/// provides a single container for all cognitive slices in the system.
#[derive(Debug)]
pub struct CognitiveFrame {
    /// Unique identifier for this frame instance.
    pub id: FrameId,

    /// Immutable anchor frame id (the "original snapshot" this frame is derived from).
    /// This is your drift shield; all deltas should ultimately reference back to this.
    pub anchor_id: FrameId,

    /// All slices belonging to this frame, keyed by SliceId.
    pub slices: HashMap<SliceId, Slice>,

    /// Creation timestamp for auditing and temporal reasoning.
    pub created_at: DateTime<Utc>,

    /// Optional metadata for UI, diagnostics, and higher-level reasoning.
    pub metadata: FrameMetadata,

    /// Ordered view of slices for flip-book style navigation.
    /// This is a logical ordering, not necessarily the same as insertion order.
    pub slice_order: Vec<SliceId>,

    /// Current position in the flip-book navigation.
    pub current_index: usize,
}

/// Errors that can occur when operating on a CognitiveFrame.
#[derive(Debug)]
pub enum CognitiveFrameError {
    SliceNotFound(SliceId),
    CellNotFound(CellId),
    NavigationOutOfRange,
    AnchorMismatch,
    InvalidOperation(&'static str),
}

pub type CognitiveResult<T> = Result<T, CognitiveFrameError>;

impl CognitiveFrame {
    /// Create a new cognitive frame with a fresh id and anchor id.
    /// In many cases, `anchor_id` will be equal to `id` for the initial root frame.
    pub fn new_root(name: Option<String>, description: Option<String>) -> Self {
        let id = FrameId::new_v4();
        let anchor_id = id;

        CognitiveFrame {
            id,
            anchor_id,
            slices: HashMap::new(),
            created_at: Utc::now(),
            metadata: FrameMetadata {
                name,
                description,
                tags: Vec::new(),
            },
            slice_order: Vec::new(),
            current_index: 0,
        }
    }

    /// Create a new cognitive frame derived from an existing anchor frame id.
    pub fn new_derived(anchor_id: FrameId, name: Option<String>, description: Option<String>) -> Self {
        let id = FrameId::new_v4();

        CognitiveFrame {
            id,
            anchor_id,
            slices: HashMap::new(),
            created_at: Utc::now(),
            metadata: FrameMetadata {
                name,
                description,
                tags: Vec::new(),
            },
            slice_order: Vec::new(),
            current_index: 0,
        }
    }

    /// Register a slice into this frame and append it to the navigation order.
    pub fn add_slice(&mut self, slice_id: SliceId, slice: Slice) {
        self.slices.insert(slice_id.clone(), slice);
        self.slice_order.push(slice_id);
    }

    /// Get an immutable reference to a slice by id.
    pub fn slice(&self, slice_id: &SliceId) -> Option<&Slice> {
        self.slices.get(slice_id)
    }

    /// Get a mutable reference to a slice by id.
    pub fn slice_mut(&mut self, slice_id: &SliceId) -> Option<&mut Slice> {
        self.slices.get_mut(slice_id)
    }

    /// Ingest a cell into a target slice.
    ///
    /// This is the basic "write" operation for the cognitive frame. Higher-level
    /// routing (e.g., deciding which slice to use for a given modality) can be
    /// implemented in a separate engine module that calls this API.
    pub fn ingest_cell(
        &mut self,
        target_slice_id: &SliceId,
        cell_id: CellId,
        cell: Cell,
    ) -> CognitiveResult<()> {
        let slice = self
            .slices
            .get_mut(target_slice_id)
            .ok_or_else(|| CognitiveFrameError::SliceNotFound(target_slice_id.clone()))?;

        slice.insert_cell(cell_id, cell);
        Ok(())
    }

    /// Apply a delta to a target slice.
    ///
    /// For now this is a simple hook; the actual delta semantics (anchor checks,
    /// conflict detection, rollback, etc.) will live in a dedicated versioning
    /// module that can call into this method.
    pub fn apply_delta<F>(
        &mut self,
        target_slice_id: &SliceId,
        f: F,
    ) -> CognitiveResult<()>
    where
        F: FnOnce(&mut Slice) -> CognitiveResult<()>,
    {
        let slice = self
            .slices
            .get_mut(target_slice_id)
            .ok_or_else(|| CognitiveFrameError::SliceNotFound(target_slice_id.clone()))?;

        f(slice)
    }

    /// Run a fact-check pass over the frame.
    ///
    /// This is a stub hook; the actual implementation will live in `layers::factcheck`
    /// and call into this method or operate directly on `self.slices`.
    pub fn run_factcheck_pass(&mut self) {
        // Intentionally left as a stub for now.
        // Later: iterate over slices, evaluate facts, write results into FactCheck slice.
    }

    /// Run a cross-reference pass over the frame.
    pub fn run_crossref_pass(&mut self) {
        // Stub: later implementation in `layers::crossref`.
    }

    /// Run a drift-control rule pass over the frame.
    pub fn run_rule_pass(&mut self) {
        // Stub: later implementation in `layers::rules`.
    }

    /// Navigate forward in the flip-book ordering.
    pub fn navigate_forward(&mut self) -> CognitiveResult<&SliceId> {
        if self.slice_order.is_empty() {
            return Err(CognitiveFrameError::NavigationOutOfRange);
        }

        if self.current_index + 1 >= self.slice_order.len() {
            return Err(CognitiveFrameError::NavigationOutOfRange);
        }

        self.current_index += 1;
        Ok(&self.slice_order[self.current_index])
    }

    /// Navigate backward in the flip-book ordering.
    pub fn navigate_backward(&mut self) -> CognitiveResult<&SliceId> {
        if self.slice_order.is_empty() {
            return Err(CognitiveFrameError::NavigationOutOfRange);
        }

        if self.current_index == 0 {
            return Err(CognitiveFrameError::NavigationOutOfRange);
        }

        self.current_index -= 1;
        Ok(&self.slice_order[self.current_index])
    }

    /// Get the currently selected slice id in the navigation order.
    pub fn current_slice_id(&self) -> Option<&SliceId> {
        self.slice_order.get(self.current_index)
    }

    /// Trace a cell across slices.
    ///
    /// This is a simple helper that scans all slices for a given CellId.
    /// Later, you can replace this with an indexed lookup in `storage::index`.
    pub fn trace_cell(&self, cell_id: &CellId) -> Vec<(&SliceId, &Cell)> {
        let mut results = Vec::new();

        for (sid, slice) in &self.slices {
            if let Some(cell) = slice.get_cell(cell_id) {
                results.push((sid, cell));
            }
        }

        results
    }
}






