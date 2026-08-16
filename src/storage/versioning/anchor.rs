// src/storage/versioning/anchor.rs
//
// Immutable anchor frame record.
// This is the "original snapshot" that all deltas ultimately reference.
// It gives you a stable base for drift control, rollback, and provenance.

use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::frame::SliceId;

/// Unique identifier for an anchor record.
pub type AnchorId = Uuid;

/// Immutable description of an anchor frame.
///
/// You can think of this as the root commit in a Git history:
/// - it never changes
/// - all deltas ultimately trace back here
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorFrame {
    /// Anchor record id (distinct from the frame id if you want).
    pub id: AnchorId,

    /// The frame id this anchor represents.
    pub frame_id: Uuid,

    /// Creation time of the anchor.
    pub created_at: DateTime<Utc>,

    /// Optional human-readable label.
    pub label: Option<String>,

    /// Optional checksum or hash of the initial frame state.
    /// You can later wire this to your folding/storage layer.
    pub checksum: Option<String>,

    /// Initial slice layout for diagnostics and replay.
    pub initial_slices: Vec<SliceId>,
}

impl AnchorFrame {
    /// Create a new anchor for a given frame id and slice layout.
    pub fn new(frame_id: Uuid, initial_slices: Vec<SliceId>, label: Option<String>) -> Self {
        Self {
            id: AnchorId::new_v4(),
            frame_id,
            created_at: Utc::now(),
            label,
            checksum: None,
            initial_slices,
        }
    }

    /// Attach a checksum after persisting the initial frame state.
    pub fn with_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.checksum = Some(checksum.into());
        self
    }
}






