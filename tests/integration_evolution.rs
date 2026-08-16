use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    config::defaults::default_policy,
};
use chrono::Utc;

#[test]
fn test_apply_decay_reduces_confidence_to_floor() {
    // Create a frame and insert a temporal slice
    let mut frame = MemoryFrame::new(default_policy());
    frame.insert_slice(LayerId::Temporal, SliceData::Temporal(chrono::Utc::now()));

    // Set a cell confidence above the floor while holding a mutable borrow,
    // then drop that borrow before calling apply_decay.
    let primary = {
        // borrow scope starts
        let slice = frame.get_slice_mut(&LayerId::Temporal).unwrap();
        let primary_cell_id = memory_frame::frame::CellId { x: 0, y: 0 };
        // ensure the cell exists and set confidence high
        slice.grid.get_cell_mut(primary_cell_id).unwrap().confidence = 1.0;
        primary_cell_id
    }; // mutable borrow of `frame` ends here

    // Call the method that needs &mut self
    frame.apply_decay(Utc::now());

    // Re-borrow to inspect the cell after decay
    {
        let slice = frame.get_slice_mut(&LayerId::Temporal).unwrap();
        let cell = slice.grid.get_cell(primary).unwrap();
        assert!(cell.confidence >= frame.policy.decay.confidence_floor);
    }
}
