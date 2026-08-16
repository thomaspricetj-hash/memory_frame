// src/storage/mod.rs

pub mod frame_folding;
pub mod frame_dax;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::Context;

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

pub fn serialize_frame(frame: &FrameRecord) -> Result<Vec<u8>> {
    // Use CBOR for on-disk persistence to support serde_json::Value and dynamic types.
    let bytes = serde_cbor::to_vec(frame).context("cbor serialization failed")?;
    Ok(bytes)
}

pub fn deserialize_frame(bytes: &[u8]) -> Result<FrameRecord> {
    let frame: FrameRecord = serde_cbor::from_slice(bytes).context("cbor deserialization failed")?;
    Ok(frame)
}

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

/// Helper: convert SliceData -> Option<serde_json::Value>
fn slice_data_to_json(data: &crate::frame::SliceData) -> Option<serde_json::Value> {
    use serde_json::json;
    match data {
        crate::frame::SliceData::Semantic(v) => Some(v.clone()),
        crate::frame::SliceData::Declarative(s) => Some(json!({ "text": s })),
        crate::frame::SliceData::Emotional(f) => Some(json!({ "emotion": f })),
        crate::frame::SliceData::Relational(v) => Some(json!({ "rel": v })),
        crate::frame::SliceData::Temporal(t) => Some(json!({ "time": t.to_rfc3339() })),
        crate::frame::SliceData::Visual(_) => None,
    }
}

/// Helper: convert Option<serde_json::Value> -> SliceData
fn slice_data_from_json(v: Option<serde_json::Value>) -> crate::frame::SliceData {
    match v {
        Some(val) => {
            if let Some(text) = val.get("text").and_then(|x| x.as_str()) {
                crate::frame::SliceData::Declarative(text.to_string())
            } else {
                crate::frame::SliceData::Semantic(val)
            }
        }
        None => crate::frame::SliceData::Declarative(String::new()),
    }
}

/// Adapter: convert MemoryFrame -> FrameRecord and serialize
/// This makes tests that expect to serialize a MemoryFrame work without changing the test logic.
pub fn serialize_memory_frame(mf: &crate::frame::MemoryFrame) -> Result<Vec<u8>> {
    let mut slices: Vec<SliceRecord> = Vec::with_capacity(mf.slices.len());

    for (sid, slice) in mf.slices.iter() {
        // convert cells
        let cells: Vec<CellRecord> = slice
            .grid
            .cells
            .iter()
            .map(|c| CellRecord {
                id_x: c.id.x,
                id_y: c.id.y,
                confidence: c.confidence,
                tags: c.tags.clone(),
            })
            .collect();

        // Use public fields on Grid (width/height are fields, not methods)
        let width = slice.grid.width;
        let height = slice.grid.height;

        let sr = SliceRecord {
            id: sid.to_string(),
            width,
            height,
            cells,
            data_json: slice_data_to_json(&slice.data),
        };

        slices.push(sr);
    }

    let fr = FrameRecord {
        slices,
        metadata: None,
    };

    serialize_frame(&fr)
}

/// Convenience wrapper: deserialize Vec<u8> into FrameRecord
pub fn deserialize_frame_vec(bytes: Vec<u8>) -> Result<FrameRecord> {
    deserialize_frame(&bytes)
}

/// Convert FrameRecord -> MemoryFrame (best-effort)
/// This helper reconstructs a MemoryFrame from a FrameRecord. It is conservative:
/// - It reconstructs LayerId/SliceId using LayerId::from_str_fast if available
/// - It constructs Cells and pushes them into slice.grid.cells
/// - It uses slice_data_from_json to reconstruct SliceData
pub fn frame_record_to_memory_frame(fr: FrameRecord) -> crate::frame::MemoryFrame {
    let mut mf = crate::frame::MemoryFrame::new(crate::config::defaults::default_policy());

    for sr in fr.slices.into_iter() {
        // Reconstruct LayerId / SliceId from string
        // Try to use a FromStr-like helper on LayerId if available; otherwise, fall back to a panic with clear message.
        let layer_id = match crate::layers::LayerId::from_str_fast(&sr.id) {
            Some(l) => l,
            None => panic!("Failed to reconstruct LayerId from {}", sr.id),
        };
        let slice_id: crate::frame::SliceId = layer_id;

        // Build a new Slice with reconstructed data
        let data = slice_data_from_json(sr.data_json);
        let mut slice = crate::frame::Slice::new(slice_id.clone(), data);

        // Set width/height directly on the grid (fields, not methods)
        slice.grid.width = sr.width;
        slice.grid.height = sr.height;

        // Reconstruct cells and push into grid
        for cr in sr.cells.into_iter() {
            let cell_id = crate::frame::CellId { x: cr.id_x, y: cr.id_y };
            let mut cell = crate::frame::Cell::new(cell_id);
            cell.confidence = cr.confidence;
            cell.tags = cr.tags.clone();
            // push into grid cells vector
            slice.grid.cells.push(cell);
        }

        mf.slices.insert(slice_id, slice);
    }

    mf
}

pub use frame_folding::{
    FoldedFrame,
    FoldedSlice,
    HybridDaxFrame,
    HybridBaseSlice,
    HybridDeltaSlice,
    fold_frame,
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







