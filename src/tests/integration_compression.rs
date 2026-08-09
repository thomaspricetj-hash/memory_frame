use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    config::defaults::default_policy,
    frame::CellId,
    compress_slice_max,
    decompress_slice_max,
    save_compressed_max,
    load_compressed_max,
};
use anyhow::Result;
use std::path::PathBuf;

#[test]
fn test_compression_and_merge_roundtrip() -> Result<()> {
    // --- Setup frame and slice ---
    let mut frame = MemoryFrame::new(default_policy());
    frame.insert_slice(LayerId::Relational, SliceData::Relational(vec![0.1, 0.2]));

    // Get a mutable reference to the inserted slice
    let slice = frame.get_slice_mut(&LayerId::Relational).expect("slice must exist");

    // Prepare two neighboring cells for merge
    let primary = CellId { x: 0, y: 0 };
    let other = CellId { x: 1, y: 0 };

    // Set confidences and a tag on the 'other' cell
    slice.grid.get_cell_mut(primary).unwrap().confidence = 0.95;
    slice.grid.get_cell_mut(other).unwrap().confidence = 0.96;
    slice.grid.get_cell_mut(other).unwrap().tags.push("merge_me".into());

    // --- Trigger your compaction/merge logic ---
    frame.compact_slices();

    // Verify merge: primary should have received the tag
    let primary_cell = slice.grid.get_cell(primary).unwrap();
    assert!(primary_cell.tags.contains(&"merge_me".into()), "merge tag should propagate");

    // --- In-memory compression roundtrip (BitDrop) ---
    let compressed = compress_slice_max(slice)?;
    let restored = decompress_slice_max(&compressed)?;
    assert_eq!(restored, *slice, "slice must roundtrip through BitDrop compression");

    // --- File save/load roundtrip ---
    let mut tmp = std::env::temp_dir();
    tmp.push("memory_frame_test_slice.mfdb");
    let tmp_path: PathBuf = tmp;

    // Save and load using the BitDrop helpers
    save_compressed_max(&tmp_path, slice)?;
    let loaded = load_compressed_max(&tmp_path)?;
    // Clean up file
    let _ = std::fs::remove_file(&tmp_path);

    assert_eq!(loaded, *slice, "slice must persist and restore via file roundtrip");

    Ok(())
}
