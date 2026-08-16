// src/storage/frame_dax.rs
//
// DAX (Delta + AXial) frame folding/unfolding utilities.
// Switched to CBOR (serde_cbor) for robust serialization of dynamic types
// such as serde_json::Value. BitDrop-V2 compression helpers remain supported.

use crate::storage::{FrameRecord, SliceRecord};
use anyhow::{Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_cbor; // use CBOR for on-disk/frame slice bytes
use bitdrop_v2; // assumed available in workspace/crates

/// A DAX-powered folded frame: base + deltas + multi-pass + diagonal semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaxFrame {
    pub base_slices: Vec<BaseSlice>,
    pub delta_slices: Vec<DeltaSlice>,
    pub metadata_base: Option<Vec<u8>>,
    pub metadata_delta: Option<Vec<u8>>,
    pub checksum: u64,
    /// Number of base slices originating from the initial (frame 0) base frame.
    /// Used during unfold to decide which base slices existed at a given frame index.
    pub base_count: usize,
}

impl Default for DaxFrame {
    fn default() -> Self {
        DaxFrame {
            base_slices: Vec::new(),
            delta_slices: Vec::new(),
            metadata_base: None,
            metadata_delta: None,
            checksum: 0,
            base_count: 0,
        }
    }
}

/// Base slice (first occurrence or canonical version).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseSlice {
    pub id: String,
    pub data: Vec<u8>,
    pub checksum: u64,
    pub harmonic_sig: u64,
    // Pattern / semantic hints (for pattern-tag folding, semantic routing)
    pub tags: Vec<String>,
    pub confidence: f32,
}

/// Delta slice: only what changed relative to a base slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaSlice {
    pub id: String,
    pub base_index: usize,
    pub delta_data: Vec<u8>,
    pub checksum: u64,
    pub frame_index: usize,   // temporal index for skip-region / routing
    // Confidence-weighted delta strength (0–1)
    pub confidence_delta: f32,
    // Semantic tags associated with this delta (pattern-tag folding)
    pub tags: Vec<String>,
    // Diagonal weight for Nine-Matrix hybrid routing
    pub diagonal_weight: f32,
}

impl DaxFrame {
    /// Build a DAX frame from a sequence of frames (multi-frame compression).
    /// `frames[0]` is treated as the base; subsequent frames are delta-compressed.
    pub fn fold_frames(frames: &[FrameRecord]) -> Result<Self> {
        if frames.is_empty() {
            return Err(anyhow::anyhow!("DaxFrame::fold_frames: empty frame list"));
        }

        let base_frame = &frames[0];

        // 1. Build base slices from the first frame (with semantic + confidence info)
        let mut base_slices = Vec::with_capacity(base_frame.slices.len());
        for slice in &base_frame.slices {
            let bytes = serde_cbor::to_vec(slice)
                .context("DaxFrame::fold_frames: cbor serialize base slice failed")?;

            let checksum = compute_checksum(&bytes);

            let (tags, confidence) = extract_slice_semantics(slice);
            let harmonic_sig = harmonic_signature_from_record(slice);

            base_slices.push(BaseSlice {
                id: slice.id.clone(),
                data: bytes,
                checksum,
                harmonic_sig,
                tags,
                confidence,
            });
        }

        // record how many base slices came from the initial frame
        let base_count = base_slices.len();

        // 2. Build metadata base
        let metadata_base = if let Some(meta) = &base_frame.metadata {
            Some(serde_json::to_vec(meta).context("serialize base metadata failed")?)
        } else {
            None
        };

        // 3. Build deltas for subsequent frames (multi-pass, semantic-aware, diagonal-aware)
        let mut delta_slices = Vec::new();
        let mut metadata_delta: Option<Vec<u8>> = None;

        // thresholds
        const PHASE_COHERENCE_THRESHOLD: f32 = 0.05;
        const SIGNATURE_DISTANCE_THRESHOLD: u64 = 0x00FF_FFFF;

        for (frame_idx, frame) in frames.iter().enumerate().skip(1) {
            // metadata delta (simple overwrite for now)
            if let Some(meta) = &frame.metadata {
                let meta_bytes = serde_json::to_vec(meta).context("serialize delta metadata failed")?;
                metadata_delta = Some(meta_bytes);
            }

            for slice in &frame.slices {
                let (tags, confidence) = extract_slice_semantics(slice);
                let sig = harmonic_signature_from_record(slice);
                let coherence = phase_coherence_from_record(slice);

                // find best base by harmonic signature distance
                let mut best: Option<(usize, u64)> = None;
                for (idx, b) in base_slices.iter().enumerate() {
                    let dist = b.harmonic_sig ^ sig;
                    match best {
                        None => best = Some((idx, dist)),
                        Some((_, best_dist)) => {
                            if dist < best_dist {
                                best = Some((idx, dist));
                            }
                        }
                    }
                }

                // Decide match using a combination of harmonic distance, phase coherence, and semantic overlap.
                let matched = if let Some((base_index, dist)) = best {
                    // direct harmonic + phase acceptance
                    let harmonic_ok = dist <= SIGNATURE_DISTANCE_THRESHOLD && coherence >= PHASE_COHERENCE_THRESHOLD;

                    // semantic overlap acceptance (fallback)
                    let semantic_ok = semantic_match(&base_slices[base_index].tags, &tags);

                    // diagonal semantic boost acceptance
                    let diagonal_sem_ok = diagonal_semantic_match(base_index, frame_idx, &base_slices[base_index].tags, &tags);

                    if harmonic_ok || semantic_ok || diagonal_sem_ok {
                        Some((base_index, base_slices[base_index].clone()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((base_index, base)) = matched {
                    let current_bytes = serde_cbor::to_vec(slice)
                        .context("DaxFrame::fold_frames: serialize current slice failed")?;

                    // BitDrop-V2 style: multi-pass XOR delta (simple stand-in)
                    let delta = xor_delta(&base.data, &current_bytes);

                    let mut checksum = compute_checksum(&delta);

                    // hybrid diagonal law: scale confidence by diagonal weight
                    let diag_w = diagonal_weight(base_index, frame_idx);
                    let mut confidence_delta = (confidence - base.confidence).abs();
                    confidence_delta *= diag_w;

                    // hybrid diagonal weighting mixed into checksum
                    checksum ^= diag_w.to_bits() as u64;

                    delta_slices.push(DeltaSlice {
                        id: slice.id.clone(),
                        base_index,
                        delta_data: delta,
                        checksum,
                        frame_index: frame_idx,
                        confidence_delta,
                        tags,
                        diagonal_weight: diag_w,
                    });
                } else {
                    // no semantic/base match: treat as new base (pattern-tag folding)
                    let bytes = serde_cbor::to_vec(slice)
                        .context("DaxFrame::fold_frames: serialize new base slice failed")?;

                    let checksum = compute_checksum(&bytes);
                    let harmonic_sig = sig;

                    base_slices.push(BaseSlice {
                        id: slice.id.clone(),
                        data: bytes,
                        checksum,
                        harmonic_sig,
                        tags,
                        confidence,
                    });
                }
            }
        }

        // 4. Frame-level checksum (multi-pass collapse loop root, diagonally weighted)
        let mut frame_hasher = DefaultHasher::new();
        for (idx, b) in base_slices.iter().enumerate() {
            let w = diagonal_weight(idx, 0);
            b.checksum.hash(&mut frame_hasher);
            b.harmonic_sig.hash(&mut frame_hasher);
            w.to_bits().hash(&mut frame_hasher);
        }
        for d in &delta_slices {
            let w = diagonal_weight(d.base_index, d.frame_index);
            d.checksum.hash(&mut frame_hasher);
            w.to_bits().hash(&mut frame_hasher);
        }
        if let Some(ref mb) = metadata_base {
            mb.hash(&mut frame_hasher);
        }
        if let Some(ref md) = metadata_delta {
            md.hash(&mut frame_hasher);
        }
        let checksum = frame_hasher.finish();

        Ok(DaxFrame {
            base_slices,
            delta_slices,
            metadata_base,
            metadata_delta,
            checksum,
            base_count,
        })
    }

    /// Reconstruct a specific frame index from the DAX structure.
    /// `frame_index == 0` returns the base frame; higher indices apply deltas.
    /// Uses temporal skip-region acceleration, diagonal law, and confidence-weighted application.
    /// Returns only the slices that are present at the requested frame index.
    pub fn unfold_frame(&self, frame_index: usize) -> Result<FrameRecord> {
        if frame_index == 0 {
            // base frame
            let mut slices = Vec::with_capacity(self.base_slices.len());
            for b in &self.base_slices {
                // verify base checksum
                let computed = compute_checksum(&b.data);
                if computed != b.checksum {
                    return Err(anyhow::anyhow!(
                        "BaseSlice checksum mismatch for id={}: stored={} computed={}",
                        b.id,
                        b.checksum,
                        computed
                    ));
                }

                let slice: SliceRecord = serde_cbor::from_slice(&b.data)
                    .context("DaxFrame::unfold_frame: deserialize base slice failed")?;
                slices.push(slice);
            }

            let metadata = if let Some(ref mb) = self.metadata_base {
                Some(serde_json::from_slice(mb).context("deserialize base metadata failed")?)
            } else {
                None
            };

            return Ok(FrameRecord { slices, metadata });
        }

        // multi-pass collapse loop: apply deltas up to requested frame_index
        // We'll reconstruct all base slices, but only return those that should exist
        // at the requested frame index. A base slice is considered present at frame_index
        // if:
        //  - it originated in the initial base (idx < base_count), or
        //  - there exists a delta referencing it with frame_index <= requested frame_index.
        let mut slices = Vec::new();

        // Precompute presence map for base slices
        let mut present = vec![false; self.base_slices.len()];
        for (idx, _b) in self.base_slices.iter().enumerate() {
            if idx < self.base_count {
                present[idx] = true;
            }
        }
        for d in &self.delta_slices {
            if d.frame_index <= frame_index && d.base_index < present.len() {
                present[d.base_index] = true;
            }
        }

        for (idx, base) in self.base_slices.iter().enumerate() {
            // start from base slice bytes
            let mut current_bytes = base.data.clone();

            // temporal skip-region acceleration:
            // only apply deltas whose frame_index <= requested frame_index
            for d in self
                .delta_slices
                .iter()
                .filter(|d| d.base_index == idx && d.frame_index <= frame_index)
            {
                // verify delta checksum (with diagonal weighting)
                let mut computed = compute_checksum(&d.delta_data);

                let w = diagonal_weight(d.base_index, d.frame_index);
                computed ^= w.to_bits() as u64;

                if computed != d.checksum {
                    return Err(anyhow::anyhow!(
                        "DeltaSlice checksum mismatch for id={}: stored={} computed={}",
                        d.id,
                        d.checksum,
                        computed
                    ));
                }

                // hybrid diagonal + confidence-weighted delta application:
                // apply only if combined strength passes threshold.
                let strength = d.confidence_delta * d.diagonal_weight;
                if strength > 0.0 {
                    current_bytes = xor_apply(&current_bytes, &d.delta_data);
                }
            }

            // Only include reconstructed slice if it should be present at this frame index
            if present[idx] {
                let slice: SliceRecord = serde_cbor::from_slice(&current_bytes)
                    .context("DaxFrame::unfold_frame: deserialize reconstructed slice failed")?;
                slices.push(slice);
            }
        }

        let metadata = if let Some(ref md) = self.metadata_delta {
            Some(serde_json::from_slice(md).context("deserialize delta metadata failed")?)
        } else if let Some(ref mb) = self.metadata_base {
            Some(serde_json::from_slice(mb).context("deserialize base metadata failed")?)
        } else {
            None
        };

        Ok(FrameRecord { slices, metadata })
    }

    /// Optional: multi-pass collapse over all frames to produce a fully collapsed state.
    /// Uses diagonal law implicitly via unfold_frame(max_index).
    pub fn collapse_all(&self) -> Result<FrameRecord> {
        // treat the highest frame_index as the final collapsed state
        let max_index = self
            .delta_slices
            .iter()
            .map(|d| d.frame_index)
            .max()
            .unwrap_or(0);
        self.unfold_frame(max_index)
    }

    // -------------------------------------------------------------------------
    // Serialization and BitDrop helpers
    // -------------------------------------------------------------------------

    /// Serialize DaxFrame to bytes using CBOR.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let raw = serde_cbor::to_vec(self).context("DaxFrame::to_bytes: cbor serialize failed")?;
        Ok(raw)
    }

    /// Deserialize DaxFrame from bytes using CBOR.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let df: DaxFrame = serde_cbor::from_slice(bytes).context("DaxFrame::from_bytes: cbor deserialize failed")?;
        Ok(df)
    }

    /// Compress serialized DaxFrame using BitDrop-V2 (infallible API assumed).
    pub fn compress_bitdrop(&self) -> Result<Vec<u8>> {
        let raw = self.to_bytes()?;
        // bitdrop_v2::compress assumed to return Vec<u8>
        let compressed = bitdrop_v2::compress(&raw);
        Ok(compressed)
    }

    /// Decompress bytes produced by compress_bitdrop and deserialize into DaxFrame.
    pub fn decompress_bitdrop(bytes: &[u8]) -> Result<Self> {
        // bitdrop_v2::decompress assumed to return Vec<u8>
        let raw = bitdrop_v2::decompress(bytes);
        let df = DaxFrame::from_bytes(&raw)?;
        Ok(df)
    }
}

/// Extract semantic tags and confidence from a slice.
/// For now, uses cell tags and average confidence as a stand-in.
fn extract_slice_semantics(slice: &SliceRecord) -> (Vec<String>, f32) {
    let mut tags = Vec::new();
    let mut total_conf = 0.0;
    let mut count = 0;

    for cell in &slice.cells {
        total_conf += cell.confidence;
        count += 1;
        for t in &cell.tags {
            if !tags.contains(t) {
                tags.push(t.clone());
            }
        }
    }

    let avg_conf = if count > 0 {
        total_conf / (count as f32)
    } else {
        0.0
    };

    (tags, avg_conf)
}

/// Compute a compact harmonic signature from a storage SliceRecord.
fn harmonic_signature_from_record(slice: &SliceRecord) -> u64 {
    let mut hasher = DefaultHasher::new();

    let len = slice.cells.len().max(1);
    let avg_conf: f32 = slice.cells.iter().map(|c| c.confidence).sum::<f32>() / len as f32;
    (avg_conf.to_bits()).hash(&mut hasher);

    let take = (len / 4).max(1);
    let low_avg: f32 = slice.cells.iter().take(take).map(|c| c.confidence).sum::<f32>() / take as f32;
    (low_avg.to_bits()).hash(&mut hasher);

    for cell in &slice.cells {
        for t in &cell.tags {
            t.hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// Approximate phase coherence from a storage SliceRecord (fallback when explicit phases are not present).
fn phase_coherence_from_record(slice: &SliceRecord) -> f32 {
    if slice.cells.is_empty() {
        return 0.0;
    }
    let avg_abs: f32 = slice.cells.iter().map(|c| c.confidence.abs()).sum::<f32>() / slice.cells.len() as f32;
    avg_abs
}

/// Simple semantic match: any overlapping tag.
fn semantic_match(base_tags: &[String], delta_tags: &[String]) -> bool {
    base_tags.iter().any(|t| delta_tags.contains(t))
}

/// Diagonal semantic match: boosts matches that align along Nine-Matrix-style diagonals.
/// Here we approximate diagonal influence using index + frame relationships.
fn diagonal_semantic_match(
    base_index: usize,
    frame_index: usize,
    base_tags: &[String],
    delta_tags: &[String],
) -> bool {
    if !semantic_match(base_tags, delta_tags) {
        return false;
    }
    let w = diagonal_weight(base_index, frame_index);
    w > 0.5
}

// -------------------------------------------------------------------------
// Helper utilities used by DAX (simple implementations included here)
// -------------------------------------------------------------------------

/// Hybrid diagonal weight for Nine-Matrix law.
/// Approximates diagonal strength using base_index and frame_index.
/// When base_index and frame_index are close, we treat it as "diagonal".
fn diagonal_weight(base_index: usize, frame_index: usize) -> f32 {
    let diff = (base_index as isize - frame_index as isize).abs() as f32;
    if diff == 0.0 {
        1.0
    } else if diff == 1.0 {
        0.75
    } else if diff == 2.0 {
        0.5
    } else {
        0.25
    }
}

/// Simple XOR-based delta between two byte buffers.
/// If lengths differ, the extra bytes are appended as-is.
fn xor_delta(base: &[u8], current: &[u8]) -> Vec<u8> {
    let len = base.len().min(current.len());
    let mut out = Vec::with_capacity(current.len());

    for i in 0..len {
        out.push(base[i] ^ current[i]);
    }

    if current.len() > len {
        out.extend_from_slice(&current[len..]);
    }

    out
}

/// Apply XOR delta to a base buffer to reconstruct the current buffer.
fn xor_apply(base: &[u8], delta: &[u8]) -> Vec<u8> {
    let len = base.len().min(delta.len());
    let mut out = Vec::with_capacity(base.len().max(delta.len()));

    for i in 0..len {
        out.push(base[i] ^ delta[i]);
    }

    if delta.len() > len {
        out.extend_from_slice(&delta[len..]);
    } else if base.len() > len {
        out.extend_from_slice(&base[len..]);
    }

    out
}

/// Compute a stable checksum for a byte slice using DefaultHasher.
fn compute_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}





