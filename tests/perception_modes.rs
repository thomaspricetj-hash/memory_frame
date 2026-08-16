// tests/perception_modes.rs
// Tests for the new perception system in MemoryFrame.
// Uses ONLY public API: set_perception_mode(), perceived_slice_order(), weight_for(), navigate_perceived().

use memory_frame::frame::memory_frame::{MemoryFrame, PerceptionMode};
use memory_frame::frame::{SliceData, CellId};
use memory_frame::layers::LayerId;
use memory_frame::config::defaults::default_policy;

#[test]
fn perception_modes_apply_weights() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    frame.insert_slice(LayerId::Visual, SliceData::Visual(vec![1, 2, 3]));
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"topic": "alpha"})));
    frame.insert_slice(LayerId::Temporal, SliceData::Temporal(chrono::Utc::now()));
    frame.insert_slice(LayerId::Declarative, SliceData::Declarative("hello".into()));

    if let Some(slice) = frame.get_slice_mut(&LayerId::Visual) {
        if let Some(cell) = slice.grid.get_cell_mut(CellId { x: 5, y: 5 }) {
            cell.confidence = 1.0;
            cell.add_tag("hot");
        }
    }

    let modes = [
        PerceptionMode::Default,
        PerceptionMode::SemanticFocus,
        PerceptionMode::TemporalFocus,
        PerceptionMode::DiagonalFocus,
        PerceptionMode::CrossConnectFocus,
    ];

    for mode in modes {
        frame.set_perception_mode(mode);

        for sid in frame.list_slices() {
            let w = frame.perception_transform.weight_for(&sid);
            assert!(w >= 0.1);
        }
    }
}

#[test]
fn perceived_slice_order_changes_with_mode() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    frame.insert_slice(LayerId::Visual, SliceData::Visual(vec![1]));
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"topic": "rich"})));
    frame.insert_slice(LayerId::Declarative, SliceData::Declarative("plain".into()));

    if let Some(slice) = frame.get_slice_mut(&LayerId::Semantic) {
        for cell in slice.grid.cells.iter_mut().take(20) {
            cell.add_tag("alpha");
            cell.confidence = 0.9;
        }
    }

    frame.set_perception_mode(PerceptionMode::Default);
    let order_default = frame.perceived_slice_order();

    frame.set_perception_mode(PerceptionMode::SemanticFocus);
    let order_semantic = frame.perceived_slice_order();

    assert_ne!(order_default, order_semantic);
    assert_eq!(order_semantic.first().unwrap(), &LayerId::Semantic);
}

#[test]
fn navigate_perceived_respects_ordering() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    frame.insert_slice(LayerId::Visual, SliceData::Visual(vec![1]));
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"topic": "x"})));
    frame.insert_slice(LayerId::Temporal, SliceData::Temporal(chrono::Utc::now()));

    if let Some(slice) = frame.get_slice_mut(&LayerId::Temporal) {
        for cell in slice.grid.cells.iter_mut().take(10) {
            cell.confidence = 1.0;
        }
    }

    frame.set_perception_mode(PerceptionMode::DiagonalFocus);

    let order = frame.perceived_slice_order();
    let first = order.first().cloned().unwrap();

    // FIX: clone first so it can be used again later
    let next = frame.navigate_perceived(
        Some(first.clone()),
        memory_frame::frame::NavTarget::NextSlice,
    );

    assert!(next.is_some());
    assert_ne!(next.unwrap(), first);
}

#[test]
fn perception_weights_are_deterministic() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    frame.insert_slice(LayerId::Visual, SliceData::Visual(vec![1]));
    frame.insert_slice(LayerId::Semantic, SliceData::Semantic(serde_json::json!({"topic": "x"})));

    frame.set_perception_mode(PerceptionMode::SemanticFocus);

    let w1 = frame.perception_transform.weight_for(&LayerId::Semantic);
    let w2 = frame.perception_transform.weight_for(&LayerId::Semantic);

    assert_eq!(w1, w2);
}
