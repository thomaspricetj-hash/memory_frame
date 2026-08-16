// src/compression/bitdrop_max.rs
//
// Production-ready BitDrop-V2 integration for memory_frame.
// Assumes bitdrop_v2::compress(&[u8]) -> Vec<u8> and
// bitdrop_v2::decompress(&[u8]) -> Vec<u8> (infallible).
//
// Ensure `Slice` implements serde::Serialize + serde::Deserialize + PartialEq + Debug.

use anyhow::{bail, Context, Result};
use bincode;
use chrono;
use std::path::Path;

use bitdrop_v2;

use crate::frame::{CellId, Slice};

/// Merge `other` cell into `primary` using diagonal law, semantic boost,
/// confidence blending, and collapse influence.
pub fn merge_cells_into_primary(slice: &mut Slice, primary: CellId, other: CellId) {
    // --- 1. Read other cell immutably ---
    let (other_conf, other_tags, other_meta) = match slice.grid.get_cell(other) {
        Some(oc) => (oc.confidence, oc.tags.clone(), oc.metadata.clone()),
        None => return,
    };

    // --- 2. Compute diagonal propagation weight ---
    let diag_w = diagonal_weight(primary, other);

    // --- 3. Compute semantic boost (NOW USED) ---
    let semantic_boost = semantic_alignment_boost(&other_tags);

    // --- 4. Compute collapse influence ---
    let collapse_inf = collapse_influence(other_conf, diag_w);

    // --- 5. Mutate primary cell ---
    if let Some(pc) = slice.grid.get_cell_mut(primary) {
        // Confidence blending (diagonal + collapse + semantic)
        let blended_conf = blended_confidence(
            pc.confidence,
            other_conf,
            diag_w,
            collapse_inf,
            semantic_boost, // semantic boost now used
        );

        pc.confidence = pc.confidence.max(blended_conf);

        // Merge tags with diagonal semantic boost
        for tag in other_tags {
            if !pc.tags.contains(&tag) {
                pc.tags.push(tag);
            }
        }

        // Metadata propagation (only if primary has none)
        if pc.metadata.is_none() && other_meta.is_some() {
            pc.metadata = other_meta;
        }

        // Temporal alignment
        pc.last_updated = Some(chrono::Utc::now());
    }
}

/// Diagonal weight based on Nineâ€‘Matrix directional distance.
fn diagonal_weight(a: CellId, b: CellId) -> f32 {
    let dx = (a.x as f32 - b.x as f32).abs();
    let dy = (a.y as f32 - b.y as f32).abs();
    let diag_dist = (dx - dy).abs();

    let w = 1.0 / (1.0 + diag_dist);
    w.clamp(0.25, 1.25)
}

/// Semantic boost: stronger when tag count is high.
fn semantic_alignment_boost(tags: &[String]) -> f32 {
    let count = tags.len() as f32;
    (1.0 + (count / 10.0)).clamp(1.0, 1.5)
}

/// Collapse influence: stronger when confidence is high and diagonal weight is strong.
fn collapse_influence(conf: f32, diag_w: f32) -> f32 {
    (conf * diag_w).clamp(0.1, 2.0)
}

/// Blend confidence using diagonal weight, collapse influence, and semantic boost.
fn blended_confidence(
    base: f32,
    other: f32,
    diag_w: f32,
    collapse_inf: f32,
    semantic_boost: f32,
) -> f32 {
    let raw = (other * diag_w * collapse_inf * semantic_boost).clamp(0.0, 1.0);
    base.max(raw)
}

//
// ---------------- BitDrop-V2 "max" integration helpers ----------------
//

/// Header format for stored blobs: 4 bytes magic + 1 byte version
const MAGIC: &[u8; 4] = b"MFDB";
const VERSION: u8 = 1;

/// Serialize `slice` with bincode and compress with BitDrop-V2.
/// This function represents the "max" compression path (single-call to the
/// compressor). Returns the headered compressed blob.
pub fn compress_slice_max(slice: &Slice) -> Result<Vec<u8>> {
    // Serialize to compact binary
    let raw = bincode::serialize(slice).context("bincode serialization failed")?;

    // Compress using BitDrop-V2 (infallible API returning Vec<u8>)
    // If your bitdrop_v2 API changes to return Result, replace this call accordingly.
    let compressed: Vec<u8> = bitdrop_v2::compress(&raw);

    // Prepend header (magic + version)
    let mut out = Vec::with_capacity(5 + compressed.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Decompress a headered blob produced by `compress_slice_max` and return the Slice.
pub fn decompress_slice_max(bytes: &[u8]) -> Result<Slice> {
    if bytes.len() < 5 || &bytes[0..4] != MAGIC {
        bail!("invalid or missing memory_frame header");
    }
    let version = bytes[4];
    if version != VERSION {
        bail!("unsupported memory_frame version: {}", version);
    }
    let body = &bytes[5..];

    // Decompress using BitDrop-V2 (infallible API returning Vec<u8>)
    let raw: Vec<u8> = bitdrop_v2::decompress(body);

    let slice: Slice = bincode::deserialize(&raw).context("bincode deserialization failed")?;
    Ok(slice)
}

/// Save compressed slice to disk using the "max" compressor.
pub fn save_compressed_max(path: &Path, slice: &Slice) -> Result<()> {
    let bytes = compress_slice_max(slice)?;
    std::fs::write(path, bytes).context("writing compressed slice to disk failed")?;
    Ok(())
}

/// Load compressed slice from disk and decompress using the "max" compressor.
pub fn load_compressed_max(path: &Path) -> Result<Slice> {
    let bytes = std::fs::read(path).context("reading compressed slice from disk failed")?;
    decompress_slice_max(&bytes)
}

/// Convenience: compress in-memory and return raw/compressed lengths and ratio for logging.
pub fn compress_and_report(slice: &Slice) -> Result<(usize, usize, f32)> {
    let raw = bincode::serialize(slice).context("bincode serialization failed")?;
    let raw_len = raw.len();

    let compressed = bitdrop_v2::compress(&raw);
    let comp_len = compressed.len();

    let ratio = if comp_len == 0 {
        0.0
    } else {
        raw_len as f32 / comp_len as f32
    };
    Ok((raw_len, comp_len, ratio))
}

//
// ---------------- Example integration points ----------------
//

/// Example: persist a slice to disk using the "max" compressor.
pub fn persist_slice_to_file(path: &Path, slice: &Slice) -> Result<()> {
    save_compressed_max(path, slice)
}

/// Example: load a slice from disk using the "max" compressor.
pub fn restore_slice_from_file(path: &Path) -> Result<Slice> {
    load_compressed_max(path)
}







