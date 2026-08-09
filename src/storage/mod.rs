// src/storage/mod.rs

pub mod frame_folding;
pub mod frame_dax;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CellRecord {
    pub id_x: usize,
    pub id_y: usize,
    pub confidence: f32,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SliceRecord {
    pub id: String,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<CellRecord>,
    pub data_json: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameRecord {
    pub slices: Vec<SliceRecord>,
    pub metadata: Option<serde_json::Value>,
}

/// Serialize a FrameRecord to bytes (bincode).
pub fn serialize_frame(frame: &FrameRecord) -> Result<Vec<u8>> {
    let bytes = bincode::serialize(frame)?;
    Ok(bytes)
}

/// Deserialize bytes into a FrameRecord.
pub fn deserialize_frame(bytes: &[u8]) -> Result<FrameRecord> {
    let frame: FrameRecord = bincode::deserialize(bytes)?;
    Ok(frame)
}

/// Simple in-memory store for tests and ephemeral usage.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    pub frames: HashMap<String, FrameRecord>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            frames: HashMap::new(),
        }
    }

    pub fn put(&mut self, key: String, frame: FrameRecord) {
        self.frames.insert(key, frame);
    }

    pub fn get(&self, key: &str) -> Option<FrameRecord> {
        self.frames.get(key).cloned()
    }
}

/// Thin wrapper around sled for a simple KV store.
pub struct KvStore {
    db: sled::Db,
}

impl KvStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key.as_bytes())? {
            Some(ivec) => Ok(Some(ivec.to_vec())),
            None => Ok(None),
        }
    }
}

/// Re-export folded and DAX types and helpers for crate-wide use.
pub use frame_folding::{
    FoldedFrame,
    FoldedSlice,
    HybridDaxFrame,
    HybridBaseSlice,
    HybridDeltaSlice,
    fold_frame,
    // keep helper functions available if needed
    diagonal_weight_for_slice,
    diagonal_weight_for_delta,
    harmonic_signature_from_record,
    phase_coherence_from_record,
    extract_confidence_from_bytes,
};

pub use frame_dax::{
    DaxFrame,
    BaseSlice,
    DeltaSlice,
};


