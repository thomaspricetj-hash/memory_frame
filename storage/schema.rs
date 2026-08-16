use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::frame::{SliceId, CellId};
use std::hash::{Hash, Hasher, DefaultHasher};

/// ReferenceLink is an extensible, typed pointer used for associative linking
/// across Cells, Slices, and Frames. It is intentionally compact and serializable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceLink {
    /// Reference to a specific cell inside a slice.
    Cell { slice_id: SliceId, cell_id: CellId },

    /// Reference to another slice by id.
    Slice { slice_id: SliceId },

    /// Reference to another frame by UUID.
    Frame { frame_id: Uuid },

    /// Semantic reference (freeform tag or ontology id).
    Semantic { tag: String },

    /// Temporal reference (e.g., "previous", "next", "delta:3", or explicit frame id).
    Temporal { label: String },

    /// Generic external reference (URI, resource id).
    External { uri: String },
}

impl ReferenceLink {
    /// Compute a stable u64 hash for the reference link (used in checksums).
    pub fn stable_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        match self {
            ReferenceLink::Cell { slice_id, cell_id } => {
                // Assume SliceId and CellId implement Hash
                slice_id.hash(&mut hasher);
                cell_id.hash(&mut hasher);
                "cell".hash(&mut hasher);
            }
            ReferenceLink::Slice { slice_id } => {
                slice_id.hash(&mut hasher);
                "slice".hash(&mut hasher);
            }
            ReferenceLink::Frame { frame_id } => {
                frame_id.hash(&mut hasher);
                "frame".hash(&mut hasher);
            }
            ReferenceLink::Semantic { tag } => {
                tag.hash(&mut hasher);
                "semantic".hash(&mut hasher);
            }
            ReferenceLink::Temporal { label } => {
                label.hash(&mut hasher);
                "temporal".hash(&mut hasher);
            }
            ReferenceLink::External { uri } => {
                uri.hash(&mut hasher);
                "external".hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// A single cell inside a slice.
/// Now includes:
/// - timestamp
/// - checksum
/// - version
/// - references (associative links)
#[derive(Debug, Serialize, Deserialize)]
pub struct CellRecord {
    pub id: CellId,
    pub confidence: f32,
    pub tags: Vec<String>,

    /// When this cell was last updated.
    pub timestamp: DateTime<Utc>,

    /// Lightweight checksum for corruption detection.
    pub checksum: u32,

    /// Version for future evolution.
    pub version: u32,

    /// Associative reference links for this cell.
    pub references: Vec<ReferenceLink>,
}

impl CellRecord {
    /// Recompute checksum from id, confidence, tags, timestamp, version, and references.
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = DefaultHasher::new();
        self.id.x.hash(&mut hasher);
        self.id.y.hash(&mut hasher);
        self.confidence.to_bits().hash(&mut hasher);
        self.timestamp.timestamp_millis().hash(&mut hasher);
        self.version.hash(&mut hasher);
        for t in &self.tags {
            t.hash(&mut hasher);
        }
        // include references in checksum
        for r in &self.references {
            r.stable_hash().hash(&mut hasher);
        }
        (hasher.finish() & 0xFFFF_FFFF) as u32
    }

    /// Verify that the stored checksum matches the computed checksum.
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.compute_checksum()
    }

    /// Diagonal weight for this cell in a Nine-Matrix sense.
    /// Uses its coordinates to approximate diagonal strength.
    pub fn diagonal_weight(&self, grid_width: usize, grid_height: usize) -> f32 {
        let cx = grid_width as f32 / 2.0;
        let cy = grid_height as f32 / 2.0;
        let dx = (self.id.x as f32 - cx).abs();
        let dy = (self.id.y as f32 - cy).abs();
        let diag_dist = (dx - dy).abs();
        let w = 1.0 / (1.0 + diag_dist);
        w.clamp(0.25, 1.0)
    }

    /// Add a reference link to this cell if it does not already exist.
    pub fn add_reference(&mut self, link: ReferenceLink) {
        if !self.references.contains(&link) {
            self.references.push(link);
            self.checksum = self.compute_checksum();
        }
    }

    /// Remove a reference link from this cell.
    pub fn remove_reference(&mut self, link: &ReferenceLink) -> bool {
        if let Some(pos) = self.references.iter().position(|r| r == link) {
            self.references.swap_remove(pos);
            self.checksum = self.compute_checksum();
            true
        } else {
            false
        }
    }

    /// Check whether a given reference exists on this cell.
    pub fn has_reference(&self, link: &ReferenceLink) -> bool {
        self.references.contains(link)
    }
}

/// A slice inside a frame.
/// Now includes:
/// - semantic checksum
/// - slice timestamp
/// - slice version
/// - cell_count for fast metadata
/// - references (associative links)
#[derive(Debug, Serialize, Deserialize)]
pub struct SliceRecord {
    pub id: SliceId,

    /// JSON or encoded payload (unchanged for compatibility).
    pub data: String,

    pub width: usize,
    pub height: usize,
    pub cells: Vec<CellRecord>,

    /// Timestamp for slice creation/update.
    pub timestamp: DateTime<Utc>,

    /// Checksum of the slice payload.
    pub checksum: u32,

    /// Number of cells (cached for fast access).
    pub cell_count: usize,

    /// Version for future evolution.
    pub version: u32,

    /// Associative reference links for this slice.
    pub references: Vec<ReferenceLink>,
}

impl SliceRecord {
    /// Recompute cell_count from cells.
    pub fn compute_cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Recompute checksum from data, cells, timestamp, version, and references.
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = DefaultHasher::new();
        // Note: assume SliceId has x/y for hashing compatibility
        self.id.x.hash(&mut hasher);
        self.id.y.hash(&mut hasher);
        self.data.hash(&mut hasher);
        self.timestamp.timestamp_millis().hash(&mut hasher);
        self.version.hash(&mut hasher);

        for cell in &self.cells {
            cell.id.x.hash(&mut hasher);
            cell.id.y.hash(&mut hasher);
            cell.confidence.to_bits().hash(&mut hasher);
            for t in &cell.tags {
                t.hash(&mut hasher);
            }
            cell.timestamp.timestamp_millis().hash(&mut hasher);
            cell.version.hash(&mut hasher);
            // include cell references
            for r in &cell.references {
                r.stable_hash().hash(&mut hasher);
            }
        }

        // include slice-level references
        for r in &self.references {
            r.stable_hash().hash(&mut hasher);
        }

        (hasher.finish() & 0xFFFF_FFFF) as u32
    }

    /// Verify that the stored checksum matches the computed checksum.
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.compute_checksum()
    }

    /// Collect all tags from all cells (semantic surface).
    pub fn all_tags(&self) -> Vec<String> {
        let mut out = Vec::new();
        for cell in &self.cells {
            for t in &cell.tags {
                if !out.contains(t) {
                    out.push(t.clone());
                }
            }
        }
        out
    }

    /// Average confidence across all cells.
    pub fn average_confidence(&self) -> f32 {
        if self.cells.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.cells.iter().map(|c| c.confidence).sum();
        sum / self.cells.len() as f32
    }

    /// Nine-Matrix diagonal signature for this slice.
    /// Uses cell positions, confidence, tags, and references.
    pub fn diagonal_signature(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.id.x.hash(&mut hasher);
        self.id.y.hash(&mut hasher);
        self.width.hash(&mut hasher);
        self.height.hash(&mut hasher);

        for cell in &self.cells {
            cell.id.x.hash(&mut hasher);
            cell.id.y.hash(&mut hasher);
            cell.confidence.to_bits().hash(&mut hasher);
            for t in &cell.tags {
                t.hash(&mut hasher);
            }
            // include cell references in signature
            for r in &cell.references {
                r.stable_hash().hash(&mut hasher);
            }
        }

        // include slice-level references
        for r in &self.references {
            r.stable_hash().hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Add a reference link to this slice if it does not already exist.
    pub fn add_reference(&mut self, link: ReferenceLink) {
        if !self.references.contains(&link) {
            self.references.push(link);
            self.checksum = self.compute_checksum();
        }
    }

    /// Remove a reference link from this slice.
    pub fn remove_reference(&mut self, link: &ReferenceLink) -> bool {
        if let Some(pos) = self.references.iter().position(|r| r == link) {
            self.references.swap_remove(pos);
            self.checksum = self.compute_checksum();
            true
        } else {
            false
        }
    }

    /// Check whether a given reference exists on this slice.
    pub fn has_reference(&self, link: &ReferenceLink) -> bool {
        self.references.contains(link)
    }

    /// Propagate a slice-level reference down to a subset of cells based on a predicate.
    /// This is useful for reinforcing associative links along diagonal or semantic surfaces.
    pub fn propagate_reference_to_cells<F>(&mut self, link: ReferenceLink, mut predicate: F)
    where
        F: FnMut(&CellRecord) -> bool,
    {
        for cell in &mut self.cells {
            if predicate(cell) && !cell.references.contains(&link) {
                cell.references.push(link.clone());
                cell.checksum = cell.compute_checksum();
            }
        }
        self.checksum = self.compute_checksum();
    }

    /// Collect all references present in this slice (slice-level + cell-level).
    pub fn collect_all_references(&self) -> Vec<ReferenceLink> {
        let mut out = Vec::new();
        for r in &self.references {
            if !out.contains(r) {
                out.push(r.clone());
            }
        }
        for cell in &self.cells {
            for r in &cell.references {
                if !out.contains(r) {
                    out.push(r.clone());
                }
            }
        }
        out
    }
}

/// A full frame.
/// Now includes:
/// - frame timestamp
/// - frame checksum
/// - frame version (already present)
/// - slice_count for fast metadata
/// - references (associative links)
#[derive(Debug, Serialize, Deserialize)]
pub struct FrameRecord {
    pub id: Uuid,
    pub slices: Vec<SliceRecord>,

    /// Version of the frame format.
    pub version: u32,

    /// Timestamp when the frame was saved.
    pub timestamp: DateTime<Utc>,

    /// Checksum of the entire frame.
    pub checksum: u32,

    /// Cached number of slices.
    pub slice_count: usize,

    /// Associative reference links for this frame.
    pub references: Vec<ReferenceLink>,
}

impl FrameRecord {
    /// Recompute slice_count from slices.
    pub fn compute_slice_count(&self) -> usize {
        self.slices.len()
    }

    /// Recompute checksum from slices, timestamp, version, and references.
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.version.hash(&mut hasher);
        self.timestamp.timestamp_millis().hash(&mut hasher);

        for slice in &self.slices {
            slice.id.x.hash(&mut hasher);
            slice.id.y.hash(&mut hasher);
            slice.checksum.hash(&mut hasher);
            slice.cell_count.hash(&mut hasher);
            slice.version.hash(&mut hasher);
            // include slice-level references in frame checksum
            for r in &slice.references {
                r.stable_hash().hash(&mut hasher);
            }
            // include cell-level references
            for cell in &slice.cells {
                for r in &cell.references {
                    r.stable_hash().hash(&mut hasher);
                }
            }
        }

        // include frame-level references
        for r in &self.references {
            r.stable_hash().hash(&mut hasher);
        }

        (hasher.finish() & 0xFFFF_FFFF) as u32
    }

    /// Verify that the stored checksum matches the computed checksum.
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.compute_checksum()
    }

    /// Nine-Matrix diagonal weight for a given slice index.
    /// Treats frame as a 1D projection of a 3x3 / N-matrix.
    pub fn diagonal_weight_for_slice(&self, slice_index: usize) -> f32 {
        if self.slices.is_empty() {
            return 0.0;
        }
        let total = self.slices.len() as f32;
        let center = (total - 1.0) / 2.0;
        let pos = slice_index as f32;
        let dist = (pos - center).abs();
        let w = 1.0 / (1.0 + dist);
        w.clamp(0.25, 1.0)
    }

    /// Nine-Matrix diagonal frame signature.
    /// Uses slice diagonal signatures, ordering, and references.
    pub fn diagonal_frame_signature(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        self.version.hash(&mut hasher);
        self.timestamp.timestamp_millis().hash(&mut hasher);

        for (idx, slice) in self.slices.iter().enumerate() {
            let w = self.diagonal_weight_for_slice(idx);
            (w.to_bits()).hash(&mut hasher);
            slice.diagonal_signature().hash(&mut hasher);
            // include slice references in signature
            for r in &slice.references {
                r.stable_hash().hash(&mut hasher);
            }
        }

        // include frame-level references
        for r in &self.references {
            r.stable_hash().hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Average confidence across all slices (using their cells).
    pub fn average_confidence(&self) -> f32 {
        if self.slices.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut count = 0;

        for slice in &self.slices {
            for cell in &slice.cells {
                sum += cell.confidence;
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            sum / count as f32
        }
    }

    /// Collect all tags across all slices and cells.
    pub fn all_tags(&self) -> Vec<String> {
        let mut out = Vec::new();
        for slice in &self.slices {
            for cell in &slice.cells {
                for t in &cell.tags {
                    if !out.contains(t) {
                        out.push(t.clone());
                    }
                }
            }
        }
        out
    }

    /// Add a frame-level reference if it does not already exist.
    pub fn add_reference(&mut self, link: ReferenceLink) {
        if !self.references.contains(&link) {
            self.references.push(link);
            self.checksum = self.compute_checksum();
        }
    }

    /// Remove a frame-level reference.
    pub fn remove_reference(&mut self, link: &ReferenceLink) -> bool {
        if let Some(pos) = self.references.iter().position(|r| r == link) {
            self.references.swap_remove(pos);
            self.checksum = self.compute_checksum();
            true
        } else {
            false
        }
    }

    /// Propagate a frame-level reference down to slices and cells based on a predicate.
    /// This reinforces associative links across the frame.
    pub fn propagate_reference_to_slices_and_cells<F>(&mut self, link: ReferenceLink, mut predicate: F)
    where
        F: FnMut(&SliceRecord) -> bool,
    {
        for slice in &mut self.slices {
            if predicate(slice) && !slice.references.contains(&link) {
                slice.references.push(link.clone());
                slice.checksum = slice.compute_checksum();
            }
            // Optionally propagate to cells that match a simple heuristic:
            slice.propagate_reference_to_cells(link.clone(), |cell| {
                // default heuristic: propagate to high-confidence cells
                cell.confidence > 0.5
            });
        }
        self.checksum = self.compute_checksum();
    }

    /// Collect all references present in the frame (frame-level + slice-level + cell-level).
    pub fn collect_all_references(&self) -> Vec<ReferenceLink> {
        let mut out = Vec::new();
        for r in &self.references {
            if !out.contains(r) {
                out.push(r.clone());
            }
        }
        for slice in &self.slices {
            for r in &slice.references {
                if !out.contains(r) {
                    out.push(r.clone());
                }
            }
            for cell in &slice.cells {
                for r in &cell.references {
                    if !out.contains(r) {
                        out.push(r.clone());
                    }
                }
            }
        }
        out
    }

    /// Resolve semantic references into matching slice indices.
    /// Returns indices of slices that match any semantic reference tag.
    pub fn resolve_semantic_references_to_slices(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let semantic_refs: Vec<&String> = self
            .collect_all_references()
            .iter()
            .filter_map(|r| {
                if let ReferenceLink::Semantic { tag } = r {
                    Some(tag)
                } else {
                    None
                }
            })
            .collect();

        if semantic_refs.is_empty() {
            return out;
        }

        for (idx, slice) in self.slices.iter().enumerate() {
            let tags = slice.all_tags();
            for sref in &semantic_refs {
                if tags.contains(sref) {
                    out.push(idx);
                    break;
                }
            }
        }
        out
    }
}







