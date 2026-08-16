use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    config::defaults::default_policy,
    frame::CellId,
};

#[test]
fn test_conflict_resolution() {
    let mut frame = MemoryFrame::new(default_policy());

    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"tag":"car"})));

    let slice = frame.get_slice_mut(&LayerId::Semantic).unwrap();

    let a = CellId { x: 0, y: 0 };
    let b = CellId { x: 1, y: 0 };

    slice.grid.get_cell_mut(a).unwrap().confidence = 0.9;
    slice.grid.get_cell_mut(b).unwrap().confidence = 0.3;

    slice.grid.get_cell_mut(b).unwrap().tags.push("vehicle".into());

    let winner = frame.resolve_conflict(a, b).unwrap();
    assert_eq!(winner, a);

    let winner_cell = slice.grid.get_cell(a).unwrap();
    assert!(winner_cell.tags.contains(&"vehicle".into()));
}






