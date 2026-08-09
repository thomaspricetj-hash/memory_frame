use memory_frame::{
    MemoryFrame,
    LayerId,
    SliceData,
    config::defaults::default_policy,
};
use chrono::Utc;

#[test]
fn test_evolution_decay() {
    let mut frame = MemoryFrame::new(default_policy());

    frame.insert_slice(LayerId::Temporal, SliceData::Temporal(Utc::now()));

    let slice = frame.get_slice_mut(&LayerId::Temporal).unwrap();
    let cell = slice.grid.get_cell_mut(memory_frame::frame::CellId { x: 0, y: 0 }).unwrap();

    cell.confidence = 1.0;

    frame.apply_decay(Utc::now());

    assert!(cell.confidence >= frame.policy.decay.confidence_floor);
}
