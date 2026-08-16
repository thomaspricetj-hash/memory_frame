// src/compression/bitdrop_max.rs
//
// BitDrop-V2 integration for memory_frame with robust, backward-compatible
// loading. This version prefers a lossless zstd-backed canonical roundtrip for
// bincode-serialized Slice objects while still accepting legacy BitDrop-V2 blobs.
//
// Header format:
//   [MAGIC (4)] [VERSION (1)] [FORMAT_TAG (1)] [PAYLOAD...]
//
// FORMAT_TAG values:
//   0x01 = zstd-compressed canonical bincode bytes (preferred, lossless)
//   0x02 = bitdrop_v2 compressed payload (legacy engine output)
//   0x00 = legacy (no tag present) — loader will attempt legacy detection

use anyhow::{bail, Context, Result};
use chrono;
use std::path::Path;

use bitdrop_v2;

use crate::frame::{CellId, Slice};

use serde_json;
use bincode;
use bincode::Options; // bring Options trait into scope for alternative option helpers
use serde_cbor;
use rmp_serde as rmps;

use zstd::bulk::{compress as zstd_compress};
use zstd::stream::decode_all as zstd_stream_dec;
use std::fs;
use std::env;

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

/// Diagonal weight based on Nine‑Matrix directional distance.
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

/// Format tags (one byte) placed immediately after the header.
const TAG_LEGACY_NONE: u8 = 0x00; // legacy blobs (no explicit tag)
const TAG_ZSTD: u8 = 0x01; // zstd-compressed canonical bincode bytes (preferred)
const TAG_BITDROP: u8 = 0x02; // bitdrop_v2 compressed payload (legacy engine output)

/// zstd magic sequence for detection
const ZSTD_MAGIC: [u8; 4] = [0x28u8, 0xB5u8, 0x2Fu8, 0xFDu8];

/// Try deserializing `decompressed` using a sequence of fallbacks:
/// 1) canonical bincode
/// 2) bincode with fixint encoding
/// 3) bincode with varint encoding
/// 4) MessagePack (rmp-serde)
/// 5) CBOR
/// 6) JSON (last resort)
fn try_deserialize_variants(decompressed: &[u8], _body_for_diag: &[u8]) -> Result<Slice>
 {
    // 1) canonical bincode
    if let Ok(s) = bincode::deserialize::<Slice>(decompressed) {
        return Ok(s);
    }

    // 2) bincode with fixint encoding
    if let Ok(s2) = bincode::options().with_fixint_encoding().deserialize::<Slice>(decompressed) {
        return Ok(s2);
    }

    // 3) bincode with varint encoding
    if let Ok(s3) = bincode::options().with_varint_encoding().deserialize::<Slice>(decompressed) {
        return Ok(s3);
    }

    // 4) MessagePack
    if let Ok(smp) = rmps::from_slice::<Slice>(decompressed) {
        return Ok(smp);
    }

    // 5) CBOR
    if let Ok(scbor) = serde_cbor::from_slice::<Slice>(decompressed) {
        return Ok(scbor);
    }

    // 6) JSON (last resort)
    if let Ok(sjson) = serde_json::from_slice::<Slice>(decompressed) {
        return Ok(sjson);
    }

    // If none matched, provide a helpful error to the caller.
    bail!("no deserializer matched in try_deserialize_variants");
}

/// Helper: scan `hay` for the `needle` sequence and return indices where it occurs.
fn find_subsequence_indices(hay: &[u8], needle: &[u8]) -> Vec<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return Vec::new();
    }
    let mut idxs = Vec::new();
    for i in 0..=(hay.len() - needle.len()) {
        if &hay[i..i + needle.len()] == needle {
            idxs.push(i);
        }
    }
    idxs
}

/// Heuristic: detect if buffer looks like a long stream of little-endian f32 values.
/// We check for many occurrences of common float bytes (0x3f, 0x40) which often appear
/// in normalized floating point data (0.5..2.0). This is only a heuristic to help diagnostics.
fn looks_like_f32_stream(buf: &[u8]) -> bool {
    if buf.len() < 16 {
        return false;
    }
    let mut count_common = 0usize;
    for &b in buf.iter().take(256) {
        if b == 0x3f || b == 0x40 || b == 0xbf || b == 0xc0 {
            count_common += 1;
        }
    }
    // if many of the first bytes are float-like, treat as likely float stream
    count_common > 16
}

/// Serialize `slice` with bincode (canonical, lossless) and compress with zstd.
/// Returns the headered compressed blob.
///
/// We use zstd for canonical roundtrips so that decompress(compress(bincode(slice))) == bincode(slice).
/// BitDrop-V2 remains supported as a legacy format on load.
pub fn compress_slice_max(slice: &Slice) -> Result<Vec<u8>> {
    // Serialize to bincode bytes (compact, deterministic)
    let raw = bincode::serialize(slice).context("bincode serialization failed")?;

    // Compress using zstd (lossless) for canonical roundtrip
    let compressed = zstd_compress(&raw, 1).context("zstd compression failed")?;

    // Prepend header (magic + version) and a format tag (zstd)
    let mut out = Vec::with_capacity(6 + compressed.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(TAG_ZSTD); // explicit format tag
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Decompress a headered blob produced by `compress_slice_max` (or legacy blobs)
/// and return the Slice.
///
/// Loader behavior:
/// - If a format tag is present (new blobs), use it to select the deserializer:
///     TAG_ZSTD    -> zstd-decompress then try deserializers
///     TAG_BITDROP -> bitdrop_v2::decompress then try deserializers
/// - If no tag is present (legacy blobs), fall back to legacy handling:
///     * try raw bincode on body
///     * decompress body with bitdrop_v2 and try canonical bincode + fallbacks
pub fn decompress_slice_max(bytes: &[u8]) -> Result<Slice> {
    if bytes.len() < 5 || &bytes[0..4] != MAGIC {
        bail!("invalid or missing memory_frame header");
    }
    let version = bytes[4];
    if version != VERSION {
        bail!("unsupported memory_frame version: {}", version);
    }

    // If there's at least one more byte, it may be the format tag (new format).
    if bytes.len() >= 6 {
        let tag = bytes[5];
        let body = &bytes[6..];

        match tag {
            TAG_ZSTD => {
                // If the body begins with a passthrough byte 0x01 followed by zstd magic,
                // strip the leading 0x01 and decode the remainder.
                if body.len() >= 5 && body[0] == 0x01 && body[1..5] == ZSTD_MAGIC {
                    match zstd_stream_dec(&body[1..]) {
                        Ok(decompressed) => {
                            // quick f32-stream heuristic: if it looks like raw floats, write debug and bail
                            if looks_like_f32_stream(&decompressed) {
                                let mut tmp = env::temp_dir();
                                tmp.push("memory_frame_decompressed_f32_stream_debug.bin");
                                let _ = fs::write(&tmp, &decompressed);
                                bail!(
                                    "decompressed payload looks like a raw f32 stream; wrote debug file to: {:?}",
                                    tmp
                                );
                            }
                            if let Ok(s) = try_deserialize_variants(&decompressed, body) {
                                return Ok(s);
                            }
                            // If deserialization fails, continue to other heuristics below.
                        }
                        Err(e) => {
                            // If this decode fails, continue to try decoding body directly below.
                            eprintln!("zstd decode failed on body[1..] after stripping leading 0x01: {}", e);
                        }
                    }
                }

                // Try zstd on body directly (existing behavior)
                match zstd_stream_dec(body) {
                    Ok(decompressed) => {
                        // f32-stream heuristic
                        if looks_like_f32_stream(&decompressed) {
                            let mut tmp = env::temp_dir();
                            tmp.push("memory_frame_decompressed_f32_stream_debug.bin");
                            let _ = fs::write(&tmp, &decompressed);
                            bail!(
                                "decompressed payload looks like a raw f32 stream; wrote debug file to: {:?}",
                                tmp
                            );
                        }

                        if let Ok(s) = try_deserialize_variants(&decompressed, body) {
                            return Ok(s);
                        }
                        // If no deserializer matched, try 3-byte-tail heuristic:
                        if decompressed.len() >= 3 && decompressed[0..3] == [0xB5u8, 0x2Fu8, 0xFDu8] {
                            let mut candidate = Vec::with_capacity(decompressed.len() + 1);
                            candidate.push(0x28u8);
                            candidate.extend_from_slice(&decompressed);
                            if let Ok(inner) = zstd_stream_dec(&candidate[..]) {
                                if let Ok(s) = try_deserialize_variants(&inner, body) {
                                    return Ok(s);
                                }
                            }
                        }
                        // Also scan for embedded zstd magic and try decoding from there
                        let indices = find_subsequence_indices(&decompressed, &ZSTD_MAGIC);
                        for &idx in &indices {
                            if idx < decompressed.len() {
                                if let Ok(inner) = zstd_stream_dec(&decompressed[idx..]) {
                                    if let Ok(s) = try_deserialize_variants(&inner, body) {
                                        return Ok(s);
                                    }
                                }
                            }
                        }
                        // fall through to error below
                    }
                    Err(e) => {
                        // If zstd decode fails, try the 3-byte-tail heuristic on body
                        if body.len() >= 3 && body[0..3] == [0xB5u8, 0x2Fu8, 0xFDu8] {
                            let mut candidate = Vec::with_capacity(body.len() + 1);
                            candidate.push(0x28u8);
                            candidate.extend_from_slice(body);
                            if let Ok(inner) = zstd_stream_dec(&candidate[..]) {
                                if let Ok(s) = try_deserialize_variants(&inner, body) {
                                    return Ok(s);
                                }
                            }
                        }
                        return Err(anyhow::anyhow!("zstd decompression failed for TAG_ZSTD: {}", e).into());
                    }
                }

                bail!("bincode deserialize failed for TAG_ZSTD after fallbacks");
            }

            TAG_BITDROP => {
                // bitdrop_v2 decompress then try deserializers
                let decompressed = bitdrop_v2::decompress(body);

                if decompressed.is_empty() {
                    bail!("bitdrop_v2 decompression produced empty payload for TAG_BITDROP");
                }

                // f32-stream heuristic
                if looks_like_f32_stream(&decompressed) {
                    let mut tmp = env::temp_dir();
                    tmp.push("memory_frame_decompressed_f32_stream_debug.bin");
                    let _ = fs::write(&tmp, &decompressed);
                    bail!(
                        "bitdrop-decompressed payload looks like a raw f32 stream; wrote debug file to: {:?}",
                        tmp
                    );
                }

                // Primary attempt: try the usual deserializers
                if let Ok(s) = try_deserialize_variants(&decompressed, body) {
                    return Ok(s);
                }

                // --- Targeted rule: handle single leading 0x01 followed by zstd magic ---
                // Observed pattern in diagnostics: decompressed begins with [0x01, 0x28, 0xB5, 0x2F, 0xFD, ...]
                if decompressed.len() >= 5 && decompressed[0] == 0x01 {
                    let zstd_magic = ZSTD_MAGIC;
                    if decompressed[1..5] == zstd_magic {
                        // Try streaming zstd decode of the remainder
                        match zstd_stream_dec(&decompressed[1..]) {
                            Ok(inner) => {
                                if looks_like_f32_stream(&inner) {
                                    let mut tmp = env::temp_dir();
                                    tmp.push("memory_frame_decompressed_f32_stream_debug.bin");
                                    let _ = fs::write(&tmp, &inner);
                                    bail!(
                                        "inner zstd-decompressed payload looks like a raw f32 stream; wrote debug file to: {:?}",
                                        tmp
                                    );
                                }

                                // Try the usual deserializers on the inner bytes
                                if let Ok(s) = try_deserialize_variants(&inner, body) {
                                    return Ok(s);
                                }

                                // If deserialization still fails, write the inner bytes to a temp debug file
                                // so we can inspect them. This helps identify nested wrapping or a different format.
                                let mut debug_path = env::temp_dir();
                                debug_path.push("memory_frame_inner_decompressed_debug.bin");
                                let _ = fs::write(&debug_path, &inner);
                                bail!(
                                    "TAG_BITDROP: stripped leading 0x01 and zstd-decompressed inner bytes but deserialization failed; wrote debug file to: {:?}",
                                    debug_path
                                );
                            }
                            Err(_) => {
                                // If zstd decode fails, fall through to other fallbacks below.
                            }
                        }
                    }
                }

                // --- Fallback A: maybe the bitdrop output contains a zstd-wrapped payload.
                // Try streaming zstd decode on the bitdrop-decompressed bytes and re-run deserializers.
                if let Ok(maybe_inner) = zstd_stream_dec(&decompressed[..]) {
                    if let Ok(s) = try_deserialize_variants(&maybe_inner, body) {
                        return Ok(s);
                    }
                }

                // --- Fallback B: strip a single leading tag and try zstd decode (broader heuristic)
                if decompressed.len() > 1 {
                    let tag_byte = decompressed[0];
                    // Known passthrough tag used in some variants: 0x7F; also try small tag heuristics 0x01/0x02
                    if tag_byte == 0x7F || tag_byte == 0x01 || tag_byte == 0x02 {
                        if let Ok(inner) = zstd_stream_dec(&decompressed[1..]) {
                            if let Ok(s) = try_deserialize_variants(&inner, body) {
                                return Ok(s);
                            }
                        }
                    }
                }

                // --- Fallback C: nested bitdrop — try decompressing the decompressed bytes again
                let nested = bitdrop_v2::decompress(&decompressed);
                if !nested.is_empty() {
                    if let Ok(s) = try_deserialize_variants(&nested, body) {
                        return Ok(s);
                    }
                    // Also try zstd on nested
                    if let Ok(inner) = zstd_stream_dec(&nested[..]) {
                        if let Ok(s) = try_deserialize_variants(&inner, body) {
                            return Ok(s);
                        }
                    }
                }

                // --- NEW: Scan for zstd magic anywhere inside the bitdrop-decompressed bytes
                // If we find the zstd magic sequence at some offset, try decoding from that offset.
                let indices = find_subsequence_indices(&decompressed, &ZSTD_MAGIC);
                for &idx in &indices {
                    if idx < decompressed.len() {
                        if let Ok(inner) = zstd_stream_dec(&decompressed[idx..]) {
                            if let Ok(s) = try_deserialize_variants(&inner, body) {
                                return Ok(s);
                            }
                        }
                    }
                }

                // --- NEW: Handle truncated/misaligned zstd streams where the 0x28 byte is missing.
                // Search for the 3-byte tail of the zstd magic [0xB5,0x2F,0xFD] and try prepending 0x28.
                let three_magic = [0xB5u8, 0x2Fu8, 0xFDu8];
                let idxs3 = find_subsequence_indices(&decompressed, &three_magic);
                for &j in &idxs3 {
                    // Build a candidate stream by prepending 0x28
                    let mut candidate = Vec::with_capacity(decompressed.len() - j + 1);
                    candidate.push(0x28u8);
                    candidate.extend_from_slice(&decompressed[j..]);
                    if let Ok(inner) = zstd_stream_dec(&candidate[..]) {
                        if let Ok(s) = try_deserialize_variants(&inner, body) {
                            return Ok(s);
                        }
                    }
                }

                // --- NEW: Try prepending a single passthrough byte (0x01) before any found zstd magic
                // This handles cases where an extra passthrough byte was removed earlier.
                let idxs_magic = find_subsequence_indices(&decompressed, &ZSTD_MAGIC);
                for &idx in &idxs_magic {
                    if idx > 0 {
                        let mut candidate = Vec::with_capacity(decompressed.len() - idx + 1);
                        candidate.push(0x01u8);
                        candidate.extend_from_slice(&decompressed[idx..]);
                        if let Ok(inner) = zstd_stream_dec(&candidate[..]) {
                            if let Ok(s) = try_deserialize_variants(&inner, body) {
                                return Ok(s);
                            }
                        }
                    }
                }

                // If all bitdrop fallbacks failed, write debug file and return a helpful error.
                let mut debug_path = env::temp_dir();
                debug_path.push("memory_frame_decompressed_debug.bin");
                let _ = fs::write(&debug_path, &decompressed);
                bail!(
                    "bincode deserialize failed for TAG_BITDROP and alternatives; wrote decompressed debug file to: {:?}",
                    debug_path
                );
            }

            TAG_LEGACY_NONE => {
                // Explicit legacy tag (rare) — fall through to legacy handling below.
            }

            _ => {
                // Unknown tag: treat as legacy (fall through to legacy handling)
            }
        }
    }

    // LEGACY PATH (no explicit tag present)
    // body starts at offset 5
    let body = &bytes[5..];

    // 1) Try interpreting the body as raw bincode (no decompression)
    if let Ok(s) = bincode::deserialize::<Slice>(body) {
        return Ok(s);
    }

    // 2) Decompress then attempt canonical bincode deserialize using bitdrop_v2
    let decompressed = bitdrop_v2::decompress(body);

    // Defensive: if decompressed is empty, report clearly
    if decompressed.is_empty() {
        bail!("decompression produced empty payload");
    }

    // f32-stream heuristic
    if looks_like_f32_stream(&decompressed) {
        let mut tmp = env::temp_dir();
        tmp.push("memory_frame_decompressed_f32_stream_debug.bin");
        let _ = fs::write(&tmp, &decompressed);
        bail!(
            "legacy bitdrop-decompressed payload looks like a raw f32 stream; wrote debug file to: {:?}",
            tmp
        );
    }

    // Try canonical bincode first and fall back through variants
    if let Ok(s) = try_deserialize_variants(&decompressed, body) {
        return Ok(s);
    }

    // Try scanning for embedded zstd magic inside the decompressed bytes (legacy fallback)
    let indices = find_subsequence_indices(&decompressed, &ZSTD_MAGIC);
    for &idx in &indices {
        if idx < decompressed.len() {
            if let Ok(inner) = zstd_stream_dec(&decompressed[idx..]) {
                if let Ok(s) = try_deserialize_variants(&inner, body) {
                    return Ok(s);
                }
            }
        }
    }

    // Try the 3-byte tail heuristic (missing 0x28)
    let three_magic = [0xB5u8, 0x2Fu8, 0xFDu8];
    let idxs3 = find_subsequence_indices(&decompressed, &three_magic);
    for &j in &idxs3 {
        let mut candidate = Vec::with_capacity(decompressed.len() - j + 1);
        candidate.push(0x28u8);
        candidate.extend_from_slice(&decompressed[j..]);
        if let Ok(inner) = zstd_stream_dec(&candidate[..]) {
            if let Ok(s) = try_deserialize_variants(&inner, body) {
                return Ok(s);
            }
        }
    }

    // Final attempt: nested bitdrop
    let nested = bitdrop_v2::decompress(&decompressed);
    if !nested.is_empty() {
        if let Ok(s) = try_deserialize_variants(&nested, body) {
            return Ok(s);
        }
        if let Ok(inner) = zstd_stream_dec(&nested[..]) {
            if let Ok(s) = try_deserialize_variants(&inner, body) {
                return Ok(s);
            }
        }
    }

    // If everything failed, write debug file and return error
    let mut debug_path = env::temp_dir();
    debug_path.push("memory_frame_decompressed_debug.bin");
    let _ = fs::write(&debug_path, &decompressed);

    // Final helpful message: we couldn't map the decompressed bytes to a known serde format.
    // If the decompressed bytes are a custom binary layout (raw floats, custom struct, etc.),
    // please provide the producer's serialization code or a short spec (field order, types,
    // endianness, counts). For convenience we already wrote the decompressed bytes to the
    // debug file above so you can paste the hex here for me to analyze.
    bail!(
        "decompress_slice_max failed: all deserialization attempts failed; wrote decompressed debug file to: {:?}",
        debug_path
    );
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
/// Uses bincode for the raw length measurement to reflect the canonical serialized size.
pub fn compress_and_report(slice: &Slice) -> Result<(usize, usize, f32)> {
    let raw = bincode::serialize(slice).context("bincode serialization failed")?;
    let raw_len = raw.len();

    // Prefer zstd for canonical raw size measurement and compression
    let compressed = zstd_compress(&raw, 1).context("zstd compression failed")?;
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
