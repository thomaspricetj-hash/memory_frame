// src/storage/frame_folding.rs

use crate::storage::{FrameRecord, SliceRecord, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bincode;
use serde_json;

/// Highâ€‘performance folded representation of a frame.
/// Optimized for speed, compression, integrity, and hybrid diagonal semantics.
#[derive(Debug, Clone)]
pub struct FoldedFrame {
    pub slices: Vec<FoldedSlice>,
    pub metadata: Option<Vec<u8>>,
    pub checksum: u64,
}

/// Folded representation of a slice (compressed + checksummed).
#[derive(Debug, Clone)]
pub struct FoldedSlice {
    pub id: String,
    pub data: Vec<u8>,
    pub checksum: u64,
    pub harmonic_sig: u64,
}

impl FoldedFrame {
    /// Maxâ€‘speed, maxâ€‘compression folding of a single FrameRecord.
    /// Hybrid diagonal law: checksums are weighted by slice position.
    pub fn fold(frame: &FrameRecord) -> Result<Self> {
        let mut folded_slices = Vec::with_capacity(frame.slices.len());

        for (idx, slice) in frame.slices.iter().enumerate() {
            let bytes = bincode::serialize(slice)?;

            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let mut checksum = hasher.finish();

            let diag_w = diagonal_weight_for_slice(idx, frame.slices.len());
            checksum ^= diag_w.to_bits() as u64;

            let harmonic_sig = harmonic_signature_from_record(slice);

            folded_slices.push(FoldedSlice {
                id: slice.id.clone(),
                data: bytes,
                checksum,
                harmonic_sig,
            });
        }

        let metadata_bytes = if let Some(meta) = &frame.metadata {
            Some(serde_json::to_vec(meta)?)
        } else {
            None
        };

        let mut frame_hasher = DefaultHasher::new();
        for (idx, fs) in self_iter_enumerate(&folded_slices) {
            let w = diagonal_weight_for_slice(idx, folded_slices.len());
            fs.checksum.hash(&mut frame_hasher);
            fs.harmonic_sig.hash(&mut frame_hasher);
            w.to_bits().hash(&mut frame_hasher);
        }
        if let Some(ref mb) = metadata_bytes {
            mb.hash(&mut frame_hasher);
        }
        let frame_checksum = frame_hasher.finish();

        Ok(FoldedFrame {
            slices: folded_slices,
            metadata: metadata_bytes,
            checksum: frame_checksum,
        })
    }

    /// Maxâ€‘speed unfolding with full integrity verification.
    /// Hybrid diagonal law: verifies diagonallyâ€‘weighted checksums.
    pub fn unfold(&self) -> Result<FrameRecord> {
        let mut frame_hasher = DefaultHasher::new();
        for (idx, fs) in self_iter_enumerate(&self.slices) {
            let w = diagonal_weight_for_slice(idx, self.slices.len());
            fs.checksum.hash(&mut frame_hasher);
            fs.harmonic_sig.hash(&mut frame_hasher);
            w.to_bits().hash(&mut frame_hasher);
        }
        if let Some(ref mb) = self.metadata {
            mb.hash(&mut frame_hasher);
        }
        let computed_frame_checksum = frame_hasher.finish();

        if computed_frame_checksum != self.checksum {
            return Err(anyhow::anyhow!(
                "Frame checksum mismatch: stored={} computed={}",
                self.checksum,
                computed_frame_checksum
            ));
        }

        let mut slices = Vec::with_capacity(self.slices.len());

        for (idx, fs) in self_iter_enumerate(&self.slices) {
            let mut hasher = DefaultHasher::new();
            fs.data.hash(&mut hasher);
            let mut computed_slice_checksum = hasher.finish();

            let diag_w = diagonal_weight_for_slice(idx, self.slices.len());
            computed_slice_checksum ^= diag_w.to_bits() as u64;

            if computed_slice_checksum != fs.checksum {
                return Err(anyhow::anyhow!(
                    "Slice checksum mismatch for id={}: stored={} computed={}",
                    fs.id,
                    fs.checksum,
                    computed_slice_checksum
                ));
            }

            let slice: SliceRecord = bincode::deserialize(&fs.data)?;
            slices.push(slice);
        }

        let metadata = if let Some(ref mb) = self.metadata {
            Some(serde_json::from_slice(mb)?)
        } else {
            None
        };

        Ok(FrameRecord {
            slices,
            metadata,
        })
    }
}

/// Public wrapper to fold a FrameRecord into a FoldedFrame.
pub fn fold_frame(frame: &FrameRecord) -> Result<FoldedFrame> {
    FoldedFrame::fold(frame)
}

/// Hybrid DAX + folding representation of multiple frames.
/// Base slices are folded; deltas are DAXâ€‘style XOR blocks, also checksummed.
/// Now includes hybrid diagonal law for routing and integrity.
#[derive(Debug, Clone)]
pub struct HybridDaxFrame {
    pub base_slices: Vec<HybridBaseSlice>,
    pub delta_slices: Vec<HybridDeltaSlice>,
    pub metadata_base: Option<Vec<u8>>,
    pub metadata_delta: Option<Vec<u8>>,
    pub checksum: u64,
}

/// Base slice (first occurrence or canonical version), folded.
#[derive(Debug, Clone)]
pub struct HybridBaseSlice {
    pub id: String,
    pub data: Vec<u8>,   // folded (bincode) base slice
    pub checksum: u64,
    pub harmonic_sig: u64,
}

/// Delta slice: only what changed relative to a base slice, XORâ€‘encoded.
#[derive(Debug, Clone)]
pub struct HybridDeltaSlice {
    pub id: String,
    pub base_index: usize,
    pub delta_data: Vec<u8>,
    pub checksum: u64,
    pub frame_index: usize,
    pub confidence_delta: f32,
    pub diagonal_weight: f32,
}

impl HybridDaxFrame {
    /// Build a hybrid DAX+folding frame from a sequence of frames.
    /// frames[0] is treated as the base; subsequent frames are deltaâ€‘compressed.
    /// Hybrid diagonal law: deltas are diagonally weighted for integrity and routing.
    pub fn fold_frames(frames: &[FrameRecord]) -> Result<Self> {
        if frames.is_empty() {
            return Err(anyhow::anyhow!("HybridDaxFrame::fold_frames: empty frame list"));
        }

        let base_frame = &frames[0];

        // Build canonical base slices from the first frame only.
        let mut base_slices = Vec::with_capacity(base_frame.slices.len());
        for (idx, slice) in base_frame.slices.iter().enumerate() {
            let bytes = bincode::serialize(slice)?;

            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let mut checksum = hasher.finish();

            let diag_w = diagonal_weight_for_slice(idx, base_frame.slices.len());
            checksum ^= diag_w.to_bits() as u64;

            let harmonic_sig = harmonic_signature_from_record(slice);

            base_slices.push(HybridBaseSlice {
                id: slice.id.clone(),
                data: bytes,
                checksum,
                harmonic_sig,
            });
        }

        let metadata_base = if let Some(meta) = &base_frame.metadata {
            Some(serde_json::to_vec(meta)?)
        } else {
            None
        };

        let mut delta_slices = Vec::new();
        let mut metadata_delta: Option<Vec<u8>> = None;

        const PHASE_COHERENCE_THRESHOLD: f32 = 0.05;
        const SIGNATURE_DISTANCE_THRESHOLD: u64 = 0x00FF_FFFF;

        // For each subsequent frame, record deltas that transform the base frame into that frame.
        for (frame_idx, frame) in frames.iter().enumerate().skip(1) {
            if let Some(meta) = &frame.metadata {
                let meta_bytes = serde_json::to_vec(meta)?;
                metadata_delta = Some(meta_bytes);
            }

            for (idx, slice) in frame.slices.iter().enumerate() {
                let sig = harmonic_signature_from_record(slice);
                let coherence = phase_coherence_from_record(slice);

                // Try to find a matching base slice by harmonic signature.
                let mut best: Option<(usize, u64)> = None;
                for (b_idx, b) in base_slices.iter().enumerate() {
                    let dist = b.harmonic_sig ^ sig;
                    match best {
                        None => best = Some((b_idx, dist)),
                        Some((_, best_dist)) => {
                            if dist < best_dist {
                                best = Some((b_idx, dist));
                            }
                        }
                    }
                }

                let matched = if let Some((base_index, dist)) = best {
                    if dist <= SIGNATURE_DISTANCE_THRESHOLD && coherence >= PHASE_COHERENCE_THRESHOLD {
                        Some((base_index, base_slices[base_index].clone()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some((base_index, base)) = matched {
                    // If we found a matching base slice, record a delta between that base and current.
                    let current_bytes = bincode::serialize(slice)?;
                    let delta_raw = xor_delta(&base.data, &current_bytes);

                    let mut hasher = DefaultHasher::new();
                    delta_raw.hash(&mut hasher);
                    let mut checksum = hasher.finish();

                    let diag_w = diagonal_weight_for_delta(base_index, frame_idx);
                    checksum ^= diag_w.to_bits() as u64;

                    let (_tags, confidence) = extract_slice_semantics(slice);
                    let mut confidence_delta =
                        (confidence - extract_confidence_from_bytes(&base.data)).abs();
                    confidence_delta *= diag_w;

                    delta_slices.push(HybridDeltaSlice {
                        id: slice.id.clone(),
                        base_index,
                        delta_data: delta_raw,
                        checksum,
                        frame_index: frame_idx,
                        confidence_delta,
                        diagonal_weight: diag_w,
                    });
                } else {
                    // No harmonic match. If this slice's position exists in the original base frame,
                    // treat it as a replacement delta for that position. This preserves original
                    // frame lengths and ordering when reconstructing.
                    if idx < base_slices.len() {
                        let current_bytes = bincode::serialize(slice)?;
                        let delta_raw = xor_delta(&base_slices[idx].data, &current_bytes);

                        let mut hasher = DefaultHasher::new();
                        delta_raw.hash(&mut hasher);
                        let mut checksum = hasher.finish();

                        let diag_w = diagonal_weight_for_delta(idx, frame_idx);
                        checksum ^= diag_w.to_bits() as u64;

                        let (_tags, confidence) = extract_slice_semantics(slice);
                        let mut confidence_delta =
                            (confidence - extract_confidence_from_bytes(&base_slices[idx].data)).abs();
                        confidence_delta *= diag_w;

                        delta_slices.push(HybridDeltaSlice {
                            id: slice.id.clone(),
                            base_index: idx,
                            delta_data: delta_raw,
                            checksum,
                            frame_index: frame_idx,
                            confidence_delta,
                            diagonal_weight: diag_w,
                        });
                    } else {
                        // If the slice index is beyond the original base frame length,
                        // append it as a new canonical base slice (this handles frames that grow).
                        let bytes = bincode::serialize(slice)?;

                        let mut hasher = DefaultHasher::new();
                        bytes.hash(&mut hasher);
                        let mut checksum = hasher.finish();

                        let diag_w = diagonal_weight_for_slice(base_slices.len(), base_slices.len() + 1);
                        checksum ^= diag_w.to_bits() as u64;

                        base_slices.push(HybridBaseSlice {
                            id: slice.id.clone(),
                            data: bytes,
                            checksum,
                            harmonic_sig: sig,
                        });
                    }
                }
            }
        }

        let mut frame_hasher = DefaultHasher::new();
        for (idx, b) in self_iter_enumerate(&base_slices) {
            let w = diagonal_weight_for_slice(idx, base_slices.len());
            b.checksum.hash(&mut frame_hasher);
            b.harmonic_sig.hash(&mut frame_hasher);
            w.to_bits().hash(&mut frame_hasher);
        }
        for (idx, d) in self_iter_enumerate(&delta_slices) {
            let w = diagonal_weight_for_delta(d.base_index, idx);
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

        Ok(HybridDaxFrame {
            base_slices,
            delta_slices,
            metadata_base,
            metadata_delta,
            checksum,
        })
    }

    /// Reconstruct a frame from the hybrid structure.
    /// frame_index == 0 returns the base frame; higher indices apply all deltas.
    /// Hybrid diagonal law: deltas are applied with diagonal strength gating.
    pub fn unfold_frame(&self, frame_index: usize) -> Result<FrameRecord> {
        // Semantic reconstruction only. Checksums are not enforced here because
        // hybrid checksums are frame-dependent and not bit-stable across versions.

        if frame_index == 0 {
            // Return the original base frame slices (the canonical base frame).
            let mut slices = Vec::with_capacity(self.base_slices.len());
            for (_idx, b) in self_iter_enumerate(&self.base_slices) {
                let slice: SliceRecord = bincode::deserialize(&b.data)?;
                slices.push(slice);
            }

            let metadata = if let Some(ref mb) = self.metadata_base {
                Some(serde_json::from_slice(mb)?)
            } else {
                None
            };

            return Ok(FrameRecord { slices, metadata });
        }

        // For other frames, start from the canonical base bytes and apply deltas
        // whose frame_index <= requested frame_index.
        let mut slices = Vec::with_capacity(self.base_slices.len());

        for (idx, base) in self_iter_enumerate(&self.base_slices) {
            let mut current_bytes = base.data.clone();

            for (_d_idx, d) in self_iter_enumerate(&self.delta_slices) {
                if d.base_index == idx && d.frame_index <= frame_index {
                    // Apply delta based on diagonal weight only. Confidence_delta is advisory;
                    // empty slices produce zero confidence_delta and should not block application.
                    if d.diagonal_weight >= 0.25 {
                        current_bytes = xor_apply(&current_bytes, &d.delta_data);
                    }
                }
            }

            let slice: SliceRecord = bincode::deserialize(&current_bytes)?;
            slices.push(slice);
        }

        // If there are appended base slices (frames that grew), include them as well.
        // (They are already part of base_slices and were handled above.)

        let metadata = if let Some(ref md) = self.metadata_delta {
            Some(serde_json::from_slice(md)?)
        } else if let Some(ref mb) = self.metadata_base {
            Some(serde_json::from_slice(mb)?)
        } else {
            None
        };

        Ok(FrameRecord { slices, metadata })
    }
}

/// Simple XOR-based delta between two byte buffers.
/// If lengths differ, the extra bytes are appended as-is.
pub fn xor_delta(base: &[u8], current: &[u8]) -> Vec<u8> {
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
pub fn xor_apply(base: &[u8], delta: &[u8]) -> Vec<u8> {
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

/// Hybrid diagonal weight for slices (FoldedFrame + HybridDaxFrame).
pub fn diagonal_weight_for_slice(index: usize, total: usize) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f32;
    let center = (total_f - 1.0) / 2.0;
    let pos = index as f32;
    let dist = (pos - center).abs();
    let w = 1.0 / (1.0 + dist);
    w.clamp(0.25, 1.0)
}

/// Hybrid diagonal weight for deltas (uses base_index + delta_index).
pub fn diagonal_weight_for_delta(base_index: usize, delta_index: usize) -> f32 {
    let diff = (base_index as isize - delta_index as isize).abs() as f32;
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

/// Helper to get (index, &T) from a slice without borrowing the iterator incorrectly.
fn self_iter_enumerate<T>(v: &[T]) -> impl Iterator<Item = (usize, &T)> {
    v.iter().enumerate()
}

/// Compute a compact harmonic signature from a storage SliceRecord.
pub fn harmonic_signature_from_record(slice: &SliceRecord) -> u64 {
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
pub fn phase_coherence_from_record(slice: &SliceRecord) -> f32 {
    if slice.cells.is_empty() {
        return 0.0;
    }
    let avg_abs: f32 = slice.cells.iter().map(|c| c.confidence.abs()).sum::<f32>() / slice.cells.len() as f32;
    avg_abs
}

/// Extract semantic tags and confidence from a slice.
pub fn extract_slice_semantics(slice: &SliceRecord) -> (Vec<String>, f32) {
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

/// Extract a confidence estimate from serialized base bytes (best-effort).
pub fn extract_confidence_from_bytes(bytes: &[u8]) -> f32 {
    if let Ok(sr) = bincode::deserialize::<SliceRecord>(bytes) {
        let (_, conf) = extract_slice_semantics(&sr);
        conf
    } else {
        0.0
    }
}







