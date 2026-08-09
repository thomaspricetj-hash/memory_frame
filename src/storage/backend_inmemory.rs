use std::collections::HashMap;
use uuid::Uuid;
use anyhow::{Result, anyhow};

use crate::storage::{FrameRecord, serialize_frame, deserialize_frame};
use crate::frame::MemoryFrame;

/// InMemoryStore is a lightweight, safe, corruption‑aware frame store.
/// It performs validation, deduplication, and safe cloning.
#[derive(Default)]
pub struct InMemoryStore {
    frames: HashMap<Uuid, FrameRecord>,
}

impl InMemoryStore {
    /// Save a frame into the store.
    /// - serializes the frame
    /// - validates the record
    /// - overwrites existing entries safely
    pub fn save(&mut self, frame: &MemoryFrame) -> Result<()> {
        let record = serialize_frame(frame)?;

        // Basic corruption check
        if record.id != frame.id {
            return Err(anyhow!(
                "InMemoryStore: record/frame ID mismatch ({} != {})",
                record.id,
                frame.id
            ));
        }

        // Insert or replace
        self.frames.insert(frame.id, record);
        Ok(())
    }

    /// Load a frame by ID.
    /// - checks existence
    /// - clones safely
    /// - deserializes with full validation
    pub fn load(&self, id: Uuid) -> Result<MemoryFrame> {
        let record = self
            .frames
            .get(&id)
            .ok_or_else(|| anyhow!("InMemoryStore: frame {} not found", id))?;

        // Clone the record to avoid borrowing issues
        let cloned = record.clone();

        // Deserialize into a full MemoryFrame
        let frame = deserialize_frame(cloned)?;

        // Final integrity check
        if frame.id != id {
            return Err(anyhow!(
                "InMemoryStore: loaded frame has mismatched ID ({} != {})",
                frame.id,
                id
            ));
        }

        Ok(frame)
    }

    /// Check if a frame exists.
    pub fn exists(&self, id: Uuid) -> bool {
        self.frames.contains_key(&id)
    }

    /// Remove a frame safely.
    pub fn delete(&mut self, id: Uuid) -> bool {
        self.frames.remove(&id).is_some()
    }

    /// Return the number of stored frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Return true if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return all frame IDs (useful for debugging or iteration).
    pub fn ids(&self) -> impl Iterator<Item = &Uuid> {
        self.frames.keys()
    }
}
