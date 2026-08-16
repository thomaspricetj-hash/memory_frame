// src/layers/crossref.rs
//
// Cross-reference layer for the hybrid cognitive memory engine.
// Builds lightweight links between cells, slices, and concepts,
// and writes them into the CrossRef slice.

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// Simple cross-reference engine.
///
/// For now, this just demonstrates the pattern:
/// - scan Semantic + Core slices
/// - emit link cells into CrossRef.
pub struct CrossRefEngine;

impl CrossRefEngine {
    pub fn new() -> Self {
        CrossRefEngine
    }

    /// Ensure a CrossRef slice exists; create if missing.
    fn ensure_crossref_slice(frame: &mut CognitiveFrame) -> SliceId {
        let cross_id = SliceId::new_crossref(); // you will add this helper.

        if frame.slice(&cross_id).is_none() {
            let mut slice = Slice::new(cross_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::CrossRef,
                "CrossRef",
                "Links between concepts, entities, gestures, and timelines.",
            );
            frame.add_slice(cross_id.clone(), slice);
        }

        cross_id
    }

    /// Run a basic cross-reference pass:
    /// - scan Semantic + Core slices
    /// - emit simple "link" cells into CrossRef.
    pub fn run(&self, frame: &mut CognitiveFrame) -> Result<(), CognitiveFrameError> {
        let cross_slice_id = Self::ensure_crossref_slice(frame);

        let semantic_and_core: Vec<(&SliceId, &Slice)> = frame
            .slices
            .iter()
            .filter(|(_, s)| matches!(s.metadata.slice_type, SliceType::Semantic | SliceType::Core))
            .collect();

        for (sid, slice) in semantic_and_core {
            for (cell_id, cell) in slice.cells_iter() {
                let link_cell_id = CellId::new_crossref_for(cell_id);
                let link_cell = Cell {
                    id: link_cell_id.clone(),
                    cell_type: CellType::GraphNode,
                    content: CellContent::Text(format!(
                        "CrossRef: link from cell {:?} in slice {:?}",
                        cell_id, sid
                    )),
                    metadata: CellMetadata {
                        confidence: cell.metadata.confidence,
                        timestamp: Utc::now().timestamp(),
                        source: CellSource::Model,
                        tags: vec!["crossref".into()],
                    },
                };

                frame.ingest_cell(&cross_slice_id, link_cell_id, link_cell)?;
            }
        }

        Ok(())
    }
}






