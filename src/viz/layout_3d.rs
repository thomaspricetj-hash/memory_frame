use crate::storage::{KvStore, deserialize_frame, FrameRecord, Result, CellRecord};

/// Simple 3D layout representation for a frame.
#[derive(Debug, Clone)]
pub struct FrameLayout3D {
    pub positions: Vec<SlicePosition>,
}

#[derive(Debug, Clone)]
pub struct SlicePosition {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl FrameLayout3D {
    pub fn new() -> Self {
        Self { positions: Vec::new() }
    }

    /// Load a FrameRecord from a KvStore and compute a simple layout.
    pub fn load_from_store(store: &KvStore, key: &str) -> Result<Self> {
        // KvStore::get(&self, key: &str) -> Result<Option<Vec<u8>>>
        let Some(bytes) = store.get(key)? else {
            return Ok(Self::new());
        };

        // deserialize_frame(bytes: &[u8]) -> Result<FrameRecord>
        let frame = deserialize_frame(&bytes)?;
        Ok(Self::from_frame_record(&frame))
    }

    pub fn from_frame_record(frame: &FrameRecord) -> Self {
        let mut layout = Self::new();

        // Deterministic ordering
        let mut slices = frame.slices.clone();
        slices.sort_by(|a, b| a.id.cmp(&b.id));

        let n = slices.len().max(1);
        let base_radius = 6.0;

        for (i, slice) in slices.iter().enumerate() {
            let angle = (i as f32) / (n as f32) * std::f32::consts::TAU;

            // Compute average confidence from cells
            let avg_conf = average_confidence(&slice.cells);

            // Confidenceâ€‘weighted radius
            let radius = base_radius * (0.8 + avg_conf * 0.4);

            // Depth (Z) based on slice ID hash â€” stable, deterministic
            let id_str = slice.id.to_string();
            let z = depth_from_id(&id_str);

            layout.positions.push(SlicePosition {
                id: id_str,
                x: radius * angle.cos(),
                y: radius * angle.sin(),
                z,
            });
        }

        layout
    }
}

/// Compute average confidence from CellRecord list.
fn average_confidence(cells: &[CellRecord]) -> f32 {
    if cells.is_empty() {
        return 0.5;
    }
    let sum: f32 = cells.iter().map(|c| c.confidence).sum();
    (sum / cells.len() as f32).clamp(0.0, 1.0)
}

/// Deterministic Zâ€‘depth based on slice ID hash.
fn depth_from_id(id: &str) -> f32 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    let v = (h.finish() % 1000) as f32 / 1000.0; // 0.0â€“1.0
    v * 2.0 - 1.0 // map to -1.0 .. +1.0
}







