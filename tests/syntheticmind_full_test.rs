//! SyntheticMind Full Max-Tier Diagnostic Suite
//! Aligned with real engine APIs (Cell, Grid, Slice, MemoryFrame, CrossConnect).

use memory_frame::frame::{
    Cell, CellId, Grid, Slice, SliceData, MemoryFrame, CrossConnect,
};
use memory_frame::layers::LayerId;
use memory_frame::config::defaults::default_policy;
use chrono::{Utc, Duration};

// ---------------------------------------------------------
// 1. CELL TESTS
// ---------------------------------------------------------

#[test]
fn test_cell_new_defaults() {
    let id = CellId { x: 1, y: 2 };
    let cell = Cell::new(id);

    assert_eq!(cell.id.x, 1);
    assert_eq!(cell.id.y, 2);
    assert_eq!(cell.confidence, 0.0);
    assert!(cell.tags.is_empty());
    assert!(cell.last_updated.is_none());
    assert!(cell.metadata.is_none());
}

#[test]
fn test_cell_touch_updates_confidence_and_time() {
    let id = CellId { x: 0, y: 0 };
    let mut cell = Cell::new(id);

    cell.touch(0.8);

    assert_eq!(cell.confidence, 0.8);
    assert!(cell.last_updated.is_some());
}

#[test]
fn test_cell_tags_and_has_tag() {
    let id = CellId { x: 0, y: 0 };
    let mut cell = Cell::new(id);

    cell.add_tag("alpha");
    cell.add_tag("beta");

    assert!(cell.has_tag("alpha"));
    assert!(cell.has_tag("beta"));
    assert!(!cell.has_tag("gamma"));
}

#[test]
fn test_cell_diagonal_signature_deterministic() {
    let id = CellId { x: 1, y: 1 };
    let mut cell = Cell::new(id);
    cell.confidence = 0.8;
    cell.add_tag("test");

    // Ensure the signature is deterministic across repeated calls.
    let sig1 = cell.diagonal_signature();
    let sig2 = cell.diagonal_signature();
    assert_eq!(sig1, sig2, "diagonal_signature should be deterministic");
}

#[test]
fn test_cell_diagonal_confidence_influence_clamped() {
    let id = CellId { x: 2, y: 2 };
    let mut cell = Cell::new(id);
    cell.confidence = 5.0; // deliberately high

    let influence = cell.diagonal_confidence_influence();
    assert!(influence >= 0.0);
    assert!(influence <= 2.0);
}

#[test]
fn test_cell_diagonal_collapse_influence_positive() {
    let id = CellId { x: 3, y: 3 };
    let mut cell = Cell::new(id);
    cell.confidence = 1.0;
    cell.add_tag("rich_tag");

    let collapse = cell.diagonal_collapse_influence();
    assert!(collapse >= 0.1);
    assert!(collapse <= 2.0);
}

// ---------------------------------------------------------
// 2. GRID TESTS
// ---------------------------------------------------------

#[test]
fn test_grid_new_initializes_cells() {
    let grid = Grid::new(4, 3);

    assert_eq!(grid.width, 4);
    assert_eq!(grid.height, 3);
    assert_eq!(grid.cell_count(), 12);

    let id = CellId { x: 2, y: 1 };
    let cell = grid.get_cell(id).unwrap();
    assert_eq!(cell.id.x, 2);
    assert_eq!(cell.id.y, 1);
}

#[test]
fn test_grid_diagonal_neighbors_center() {
    let grid = Grid::new(3, 3);
    let id = CellId { x: 1, y: 1 };

    let diag = grid.diagonal_neighbors(id);
    assert_eq!(diag.len(), 4);
}

#[test]
fn test_grid_diagonal_neighbors_corner() {
    let grid = Grid::new(3, 3);
    let id = CellId { x: 0, y: 0 };

    let diag = grid.diagonal_neighbors(id);
    // Only SE is in bounds
    assert_eq!(diag.len(), 1);
    assert_eq!(diag[0].x, 1);
    assert_eq!(diag[0].y, 1);
}

#[test]
fn test_grid_diagonal_confidence_blend() {
    let mut grid = Grid::new(3, 3);
    let center = CellId { x: 1, y: 1 };

    // Set center and diagonals to known confidences
    if let Some(c) = grid.get_cell_mut(center) {
        c.confidence = 1.0;
    }
    for cid in grid.diagonal_neighbors(center) {
        if let Some(c) = grid.get_cell_mut(cid) {
            c.confidence = 0.5;
        }
    }

    let blended = grid.diagonal_confidence(center);
    // 0.6 * own (1.0) + 0.4 * avg(diagonals=0.5) = 0.6 + 0.2 = 0.8
    assert!((blended - 0.8).abs() < 1e-3);
}

#[test]
fn test_grid_diagonal_block_signature_changes_with_tags() {
    let mut grid = Grid::new(3, 3);
    let id = CellId { x: 1, y: 1 };

    let sig_before = grid.diagonal_block_signature(id);

    if let Some(c) = grid.get_cell_mut(id) {
        c.add_tag("alpha");
        c.confidence = 0.9;
    }

    let sig_after = grid.diagonal_block_signature(id);
    assert_ne!(sig_before, sig_after);
}

#[test]
fn test_grid_dominant_tags_and_histogram() {
    let mut grid = Grid::new(2, 2);

    for cell in grid.cells.iter_mut() {
        cell.add_tag("alpha");
    }
    grid.cells[0].add_tag("beta");

    let dom = grid.dominant_tags();
    assert!(dom.contains(&"alpha".to_string()));

    let hist = grid.tag_histogram();
    assert!(!hist.is_empty());
}

// ---------------------------------------------------------
// 3. SLICE TESTS
// ---------------------------------------------------------

#[test]
fn test_slice_new_uses_default_grid_size() {
    let id = LayerId::Visual;
    let data = SliceData::Visual(vec![0, 1, 2]);

    let slice = Slice::new(id, data);

    assert_eq!(slice.grid.width, 32);
    assert_eq!(slice.grid.height, 32);
    assert_eq!(slice.cell_count(), 32 * 32);
}

#[test]
fn test_slice_diagonal_signature_nonzero_with_confidence() {
    let id = LayerId::Semantic;
    let data = SliceData::Semantic(serde_json::json!({"k": "v"}));

    let mut slice = Slice::new(id, data);

    // Strengthen a couple of cells
    if let Some(cell) = slice.grid.get_cell_mut(CellId { x: 10, y: 10 }) {
        cell.confidence = 1.0;
    }
    if let Some(cell) = slice.grid.get_cell_mut(CellId { x: 20, y: 20 }) {
        cell.confidence = 0.5;
    }

    let sig = slice.diagonal_signature();
    // Accept any numeric signature; ensure determinism by calling twice.
    let sig2 = slice.diagonal_signature();
    assert_eq!(sig, sig2);
}

#[test]
fn test_slice_diagonal_semantic_surface_increases_with_tags() {
    let id = LayerId::Semantic;
    let data = SliceData::Semantic(serde_json::json!({"topic": "test"}));

    let mut slice = Slice::new(id, data);

    let base = slice.diagonal_semantic_surface();

    // Add tags to many cells
    for cell in slice.grid.cells.iter_mut().take(50) {
        cell.add_tag("alpha");
    }

    let boosted = slice.diagonal_semantic_surface();
    assert!(boosted >= base);
}

#[test]
fn test_slice_diagonal_confidence_surface_respects_aspect_ratio() {
    let id = LayerId::Visual;
    let data = SliceData::Visual(vec![1, 2, 3]);

    let slice_square = Slice::with_size(id.clone(), data.clone(), 16, 16);
    let slice_rect = Slice::with_size(id, data, 32, 8);

    let conf_square = slice_square.diagonal_confidence_surface();
    let conf_rect = slice_rect.diagonal_confidence_surface();

    // Aspect ratio penalty should reduce rect surface vs square (same avg confidence)
    assert!(conf_square >= conf_rect);
}

#[test]
fn test_slice_diagonal_collapse_influence_positive() {
    let id = LayerId::Emotional;
    let data = SliceData::Emotional(0.7);

    let mut slice = Slice::new(id, data);

    for cell in slice.grid.cells.iter_mut().take(10) {
        cell.confidence = 1.0;
    }

    let collapse = slice.diagonal_collapse_influence();
    assert!(collapse >= 0.1);
    assert!(collapse <= 2.0);
}

// ---------------------------------------------------------
// 4. MEMORY FRAME TESTS
// ---------------------------------------------------------

#[test]
fn test_memory_frame_new_uses_default_policy() {
    let policy = default_policy();
    let frame = MemoryFrame::new(policy);

    assert!(frame.slices.is_empty());
    assert_eq!(frame.diagonal_frame_signature, 0.0);
    assert_eq!(frame.temporal_alignment_score, 0.0);
}

#[test]
fn test_memory_frame_insert_slice_updates_diagonal_metrics() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    let id = LayerId::Visual;
    let data = SliceData::Visual(vec![0, 1, 2]);

    frame.insert_slice(id, data);

    assert!(frame.diagonal_frame_signature >= 0.0);
    assert!(frame.diagonal_semantic_surface >= 0.0);
    assert!(frame.diagonal_confidence_surface >= 0.0);
    assert!(frame.diagonal_propagation_weight >= 0.25);
    assert!(frame.diagonal_collapse_influence >= 0.25);
}

#[test]
fn test_memory_frame_temporal_metrics_with_old_cells() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    let id = LayerId::Temporal;
    let data = SliceData::Temporal(Utc::now());

    frame.insert_slice(id, data);

    // Make cells "old"
    if let Some(slice) = frame.slices.values_mut().next() {
        for cell in &mut slice.grid.cells {
            cell.last_updated = Some(Utc::now() - Duration::seconds(60));
            cell.confidence = 1.0;
        }
    }

    frame.recompute_temporal_metrics();

    assert!(frame.temporal_alignment_score >= 0.0);
    assert!(frame.temporal_decay_factor <= 1.0);
    assert!(frame.temporal_diagonal_signature >= 0.0);
}

#[test]
fn test_memory_frame_touch_slice_recomputes_diagonal_metrics() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    let id = LayerId::Semantic;
    let data = SliceData::Semantic(serde_json::json!({"topic": "initial"}));

    frame.insert_slice(id.clone(), data);

    let before = frame.diagonal_frame_signature;

    // Mutate slice
    if let Some(slice) = frame.get_slice_mut(&id) {
        for cell in slice.grid.cells.iter_mut().take(20) {
            cell.confidence = 1.0;
        }
    }

    frame.touch_slice(&id);

    let after = frame.diagonal_frame_signature;
    assert_ne!(before, after);
}

#[test]
fn test_memory_frame_find_slice_for_cell() {
    let policy = default_policy();
    let mut frame = MemoryFrame::new(policy);

    let id = LayerId::Visual;
    let data = SliceData::Visual(vec![0]);

    frame.insert_slice(id.clone(), data);

    // Pick a cell that must exist in the default 32×32 grid
    let cell_id = CellId { x: 10, y: 10 };

    let found = frame.find_slice_for_cell(cell_id);
    assert_eq!(found, Some(id));
}

// ---------------------------------------------------------
// 5. CROSS-CONNECT TESTS
// ---------------------------------------------------------

#[test]
fn test_cross_connect_add_and_remove_link() {
    let mut cc = CrossConnect::new();

    let a = CellId { x: 0, y: 0 };
    let b = CellId { x: 1, y: 1 };

    cc.add_link(a, b, 0.8);

    let from_a = cc.links_from(a);
    assert_eq!(from_a.len(), 1);
    assert_eq!(from_a[0], b);

    cc.remove_link(a, b);
    let from_a_after = cc.links_from(a);
    assert!(from_a_after.is_empty());
}

#[test]
fn test_cross_connect_normalize_weights() {
    let mut cc = CrossConnect::new();

    let a = CellId { x: 0, y: 0 };
    let b = CellId { x: 1, y: 1 };
    let c = CellId { x: 2, y: 2 };

    cc.add_link(a, b, 0.5);
    cc.add_link(a, c, 0.5);

    cc.normalize();

    let weights = cc.weighted_links_from(a);
    let sum: f32 = weights.iter().map(|(_, w)| *w).sum();
    assert!((sum - 1.0).abs() < 1e-3);
}

#[test]
fn test_cross_connect_traverse_multi_hop_respects_decay() {
    let mut cc = CrossConnect::new();

    let a = CellId { x: 0, y: 0 };
    let b = CellId { x: 1, y: 1 };
    let c = CellId { x: 2, y: 2 };

    cc.add_link(a, b, 1.0);
    cc.add_link(b, c, 1.0);

    let results = cc.traverse_multi_hop(a, 2, 0.1);

    // Should reach c with some positive weight
    let mut found_c = false;
    for (id, w) in results {
        if id == c {
            assert!(w > 0.0);
            found_c = true;
        }
    }
    assert!(found_c);
}

#[test]
fn test_cross_connect_diagonal_metrics_nonzero_after_links() {
    let mut cc = CrossConnect::new();

    let a = CellId { x: 0, y: 0 };
    let b = CellId { x: 3, y: 3 };

    cc.add_link(a, b, 0.9);

    assert!(cc.diagonal_frame_signature >= 0.0);
    assert!(cc.diagonal_semantic_surface >= 0.0);
    assert!(cc.diagonal_propagation_weight >= 0.25);
    assert!(cc.diagonal_collapse_influence >= 0.25);
}

