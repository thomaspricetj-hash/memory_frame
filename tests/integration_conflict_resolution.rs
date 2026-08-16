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

    // Insert two slices that will be compared by resolve_conflict
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"tag":"car"})));
    frame.insert_slice(LayerId::Relational, SliceData::Relational(vec![0.0_f32]));

    // Keep the slice ids for later assertions
    let a_slice = LayerId::Semantic;
    let b_slice = LayerId::Relational;

    // Mutate each slice in its own scope to avoid simultaneous mutable borrows
    {
        let mut a = frame.get_slice_mut(&a_slice).unwrap();
        let a_cell = CellId { x: 0, y: 0 };
        a.grid.get_cell_mut(a_cell).unwrap().confidence = 0.9;
    } // `a` borrow dropped here

    {
        let mut b = frame.get_slice_mut(&b_slice).unwrap();
        let b_cell = CellId { x: 0, y: 0 };
        b.grid.get_cell_mut(b_cell).unwrap().confidence = 0.3;
        b.grid.get_cell_mut(b_cell).unwrap().tags.push("vehicle".into());
    } // `b` borrow dropped here

    // Call resolve_conflict using clones so we don't move the original ids
    let winner = frame.resolve_conflict(a_slice.clone(), b_slice.clone()).unwrap();
    assert_eq!(winner, a_slice);

    // Re-borrow the winner slice and check primary cell exists
    let winner_slice = frame.get_slice_mut(&winner).unwrap();
    let primary_cell = winner_slice.grid.get_cell(CellId { x: 0, y: 0 }).unwrap();
    assert!(primary_cell.confidence >= 0.0);
}
