// src/storage/versioning/delta.rs
//
// DeltaRecord: the core unit of cognitive versioning.
// Every update to a slice or cell creates a DeltaRecord that
// references the original anchor state and describes the change.
//
// This enables:
// - rollback
// - replay
// - conflict detection
// - drift resistance
// - full cognitive traceability

use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::frame::{CellId, SliceId};
use crate::frame::cell_types::{CellType, CellContent, CellSource};

/// Unique identifier for a delta record.
pub type DeltaId = Uuid;

/// The type of change represented by a delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaKind {
    /// A new cell was added.
    Insert,

    /// An existing cell was modified.
    Update,

    /// A cell was removed.
    Delete,

    /// A semantic or structural correction.
    Correction,

    /// A contradiction or fact-check adjustment.
    Contradiction,

    /// A rule-triggered adjustment (drift control).
    RuleAdjustment,
}

/// A single delta entry describing a change to a cell or slice.
///
/// This is the cognitive equivalent of a Git commit.
/// Every delta references:
/// - the anchor frame
/// - the target slice
/// - the target cell (if applicable)
/// - the type of change
/// - the new content (if applicable)
/// - the reason for the change
/// - the source (user/model/external)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaRecord {
    /// Unique delta identifier.
    pub id: DeltaId,

    /// The immutable anchor frame this delta ultimately traces back to.
    pub anchor_frame: Uuid,

    /// The slice being modified.
    pub slice_id: SliceId,

    /// The cell being modified (optional for slice-level deltas).
    pub cell_id: Option<CellId>,

    /// The type of change.
    pub kind: DeltaKind,

    /// New content (for Insert/Update/Correction).
    pub new_content: Option<CellContent>,

    /// Optional previous content (for Update/Delete).
    pub old_content: Option<CellContent>,

    /// Human-readable reason for the change.
    pub reason: String,

    /// Confidence score for the delta.
    pub confidence: f32,

    /// Source of the delta (user/model/external).
    pub source: CellSource,

    /// Timestamp of the delta.
    pub timestamp: i64,
}

impl DeltaRecord {
    /// Create a new delta record.
    pub fn new(
        anchor_frame: Uuid,
        slice_id: SliceId,
        cell_id: Option<CellId>,
        kind: DeltaKind,
        new_content: Option<CellContent>,
        old_content: Option<CellContent>,
        reason: impl Into<String>,
        source: CellSource,
        confidence: f32,
    ) -> Self {
        Self {
            id: DeltaId::new_v4(),
            anchor_frame,
            slice_id,
            cell_id,
            kind,
            new_content,
            old_content,
            reason: reason.into(),
            confidence,
            source,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}






