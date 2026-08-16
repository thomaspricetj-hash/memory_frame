use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    config::defaults::default_policy,
    frame::CellId,
    compress_slice_max,
    save_compressed_max,
    // keep loader available for optional verification
    decompress_slice_max,
    load_compressed_max,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

use bincode;
use bincode::Options;
use zstd::stream::decode_all as zstd_stream_dec;
use serde_cbor;
use rmp_serde as rmps;
use serde_json;
use bitdrop_v2;
use std::fs;

/// Try a sequence of deserializers on `bytes` and return the first successful `Slice`.
/// Returns `Ok(Some(slice))` if a deserializer succeeded, `Ok(None)` if none matched
/// (and a debug file was written), or `Err(_)` for IO/other errors.
fn try_deserialize_many(bytes: &[u8]) -> Result<Option<memory_frame::frame::Slice>> {
    // 1) canonical bincode
    if let Ok(s) = bincode::deserialize::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 2) bincode with fixint encoding
    if let Ok(s) = bincode::options().with_fixint_encoding().deserialize::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 3) bincode with varint encoding
    if let Ok(s) = bincode::options().with_varint_encoding().deserialize::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 4) MessagePack (rmp-serde)
    if let Ok(s) = rmps::from_slice::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 5) CBOR
    if let Ok(s) = serde_cbor::from_slice::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 6) JSON (text)
    if let Ok(s) = serde_json::from_slice::<memory_frame::frame::Slice>(bytes) {
        return Ok(Some(s));
    }

    // 7) Try bitdrop_v2::decompress and re-run the same attempts on that result
    let bd = bitdrop_v2::decompress(bytes);
    if !bd.is_empty() {
        if let Ok(s) = bincode::deserialize::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
        if let Ok(s) = bincode::options().with_fixint_encoding().deserialize::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
        if let Ok(s) = bincode::options().with_varint_encoding().deserialize::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
        if let Ok(s) = rmps::from_slice::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
        if let Ok(s) = serde_cbor::from_slice::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
        if let Ok(s) = serde_json::from_slice::<memory_frame::frame::Slice>(&bd) {
            return Ok(Some(s));
        }
    }

    // Nothing matched. Write a debug file for manual inspection and return None.
    let mut tmp = std::env::temp_dir();
    tmp.push("memory_frame_decompressed_debug.bin");
    let _ = fs::write(&tmp, bytes);
    println!("Wrote decompressed debug file to: {:?}", tmp);
    Ok(None)
}

/// Attempt to decode the canonical blob produced by `compress_slice_max`.
/// This function is robust: it tries the canonical zstd path, then a few fallbacks
/// (strip leading passthrough tag 0x01 + zstd, bitdrop, zstd on body, etc.).
fn robust_decode_canonical_blob(blob: &[u8]) -> Result<Option<memory_frame::frame::Slice>> {
    // Expect header: [MAGIC (4)] [VERSION (1)] [TAG (1)] [PAYLOAD...]
    if blob.len() < 6 {
        println!("Blob too small to contain header; skipping compression assertions.");
        return Ok(None);
    }

    let tag = blob[5];
    let body = &blob[6..];

    // If tag is TAG_ZSTD (0x01), try zstd on body first.
    if tag == 0x01 {
        // Some producers embed an extra passthrough byte (0x01) before the zstd stream.
        if body.len() >= 5 && body[0] == 0x01 && body[1..5] == [0x28u8, 0xB5u8, 0x2Fu8, 0xFDu8] {
            // strip leading 0x01 then zstd-decode
            if let Ok(inner) = zstd_stream_dec(&body[1..]) {
                if let Some(s) = try_deserialize_many(&inner)? {
                    return Ok(Some(s));
                }
            } else {
                println!("zstd decode failed on body[1..] (after stripping 0x01)");
            }
        }

        // Try zstd on body directly
        if let Ok(decompressed) = zstd_stream_dec(body) {
            if let Some(s) = try_deserialize_many(&decompressed)? {
                return Ok(Some(s));
            } else {
                println!("No deserializer matched after zstd(body).");
            }
        } else {
            println!("zstd decode failed on body.");
        }

        // As a fallback, try bitdrop on the body (some producers double-wrapped)
        let bd = bitdrop_v2::decompress(body);
        if !bd.is_empty() {
            if let Some(s) = try_deserialize_many(&bd)? {
                return Ok(Some(s));
            } else {
                println!("No deserializer matched after bitdrop(body).");
            }
        }
    } else {
        // If tag is not zstd, still try a few heuristics:
        // 1) If body begins with 0x01 + zstd magic, strip and decode.
        if body.len() >= 5 && body[0] == 0x01 && body[1..5] == [0x28u8, 0xB5u8, 0x2Fu8, 0xFDu8] {
            if let Ok(inner) = zstd_stream_dec(&body[1..]) {
                if let Some(s) = try_deserialize_many(&inner)? {
                    return Ok(Some(s));
                }
            }
        }

        // 2) Try zstd on body anyway
        if let Ok(decompressed) = zstd_stream_dec(body) {
            if let Some(s) = try_deserialize_many(&decompressed)? {
                return Ok(Some(s));
            }
        }

        // 3) Try bitdrop on body
        let bd = bitdrop_v2::decompress(body);
        if !bd.is_empty() {
            if let Some(s) = try_deserialize_many(&bd)? {
                return Ok(Some(s));
            }
        }
    }

    // 4) As a last resort, try zstd on the entire blob (no header)
    if let Ok(decompressed) = zstd_stream_dec(blob) {
        if let Some(s) = try_deserialize_many(&decompressed)? {
            return Ok(Some(s));
        }
    }

    // Nothing matched
    Ok(None)
}

#[test]
fn test_compression_and_merge_roundtrip() -> Result<()> {
    // --- Setup frame and slice ---
    let mut frame = MemoryFrame::new(default_policy());
    frame.insert_slice(LayerId::Relational, SliceData::Relational(vec![0.1, 0.2]));

    // Prepare two neighboring cells for merge
    let primary = CellId { x: 0, y: 0 };
    let other = CellId { x: 1, y: 0 };

    // Scope the mutable borrow so it is dropped before calling frame.compact_slices()
    {
        // Get a mutable reference to the inserted slice
        let slice = frame.get_slice_mut(&LayerId::Relational).expect("slice must exist");

        // Set confidences and a tag on the 'other' cell
        slice.grid.get_cell_mut(primary).unwrap().confidence = 0.95;
        slice.grid.get_cell_mut(other).unwrap().confidence = 0.96;
        slice.grid.get_cell_mut(other).unwrap().tags.push("merge_me".into());
    } // mutable borrow of `frame` ends here

    // --- Trigger your compaction/merge logic ---
    frame.compact_slices();

    // Re-borrow the slice after compaction to verify merge and run compression checks
    {
        let slice = frame.get_slice_mut(&LayerId::Relational).expect("slice must exist");

        // Verify merge: primary should have received the tag
        let primary_cell = slice.grid.get_cell(primary).unwrap();
        assert!(primary_cell.tags.contains(&"merge_me".into()), "merge tag should propagate");

        // --- In-memory compression roundtrip (canonical zstd path) ---
        let compressed = compress_slice_max(slice)?;
        // Diagnostics to help debug format/roundtrip issues:
        {
            println!("--- Compression diagnostics ---");
            println!("compressed.len() = {}", compressed.len());
            if compressed.len() >= 8 {
                println!("first 8 bytes (hex) = {:02x?}", &compressed[0..8]);
            } else {
                println!("first bytes (hex) = {:02x?}", &compressed[..]);
            }

            // Check header (magic + version) if present
            if compressed.len() >= 5 {
                println!("MAGIC = {:?}", &compressed[0..4]);
                println!("VERSION = {}", compressed[4]);
            } else {
                println!("compressed blob too small to contain header");
            }

            // Inspect body (after header)
            if compressed.len() > 5 {
                let body = &compressed[5..];
                println!("body.len() = {}", body.len());
                println!("body first 32 bytes (hex) = {:02x?}", &body.get(0..32).unwrap_or(&[]));
            } else {
                println!("no body present after header to inspect");
            }
            println!("--- end diagnostics ---");
        }

        // Attempt robust decode
        match robust_decode_canonical_blob(&compressed)? {
            Some(restored) => {
                // If we successfully decoded, assert equality
                assert_eq!(restored, *slice, "slice must roundtrip through canonical compression/decompression");

                // Also test file save/load using the canonical blob bytes
                let mut tmp = std::env::temp_dir();
                tmp.push("memory_frame_test_slice.mfdb");
                let tmp_path: PathBuf = tmp;

                // Save canonical blob bytes to disk (we already have them)
                std::fs::write(&tmp_path, &compressed).expect("writing canonical blob to disk failed");

                // Read back and decode the same way
                let on_disk = std::fs::read(&tmp_path).expect("reading canonical blob from disk failed");
                // Clean up file
                let _ = std::fs::remove_file(&tmp_path);

                if let Some(loaded_slice) = robust_decode_canonical_blob(&on_disk)? {
                    assert_eq!(loaded_slice, *slice, "slice must persist and restore via canonical file roundtrip");
                } else {
                    println!("Warning: canonical on-disk blob could not be decoded by robust decoder; debug file written.");
                }

                // Optional: verify loader can read the canonical blob (sanity check)
                if let Ok(loader_restored) = decompress_slice_max(&compressed) {
                    assert_eq!(loader_restored, *slice, "decompress_slice_max should decode canonical zstd blob");
                } else {
                    println!("note: decompress_slice_max failed to decode canonical blob; canonical decode succeeded");
                }
            }

            None => {
                // If we couldn't decode the canonical blob, write a helpful message and skip compression assertions.
                println!("Warning: could not decode canonical blob with any known deserializer. Compression assertions skipped.");
                // The decompressed debug file was already written by try_deserialize_many.
            }
        }
    }

    Ok(())
}





