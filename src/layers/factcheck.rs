// src/layers/factcheck.rs
//
// Fact-check layer for the hybrid cognitive memory engine.
// Scans slices, evaluates basic consistency, and writes results
// into the FactCheck slice.

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// Simple fact-check engine.
///
/// This is intentionally conservative: it doesn't try to be smart,
/// it just demonstrates the pattern of:
/// - scanning Core/Delta
/// - emitting verdicts into FactCheck.
pub struct FactCheckEngine;

impl FactCheckEngine {
    pub fn new() -> Self {
        FactCheckEngine
    }

    /// Ensure a FactCheck slice exists; create if missing.
    fn ensure_factcheck_slice(frame: &mut CognitiveFrame) -> SliceId {
        // For now we assume SliceId is something like Uuid or String.
        // You can adapt this to your actual SliceId type.
        let fact_id = SliceId::new_factcheck(); // you will implement this helper on SliceId.

        if frame.slice(&fact_id).is_none() {
            let mut slice = Slice::new(fact_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::FactCheck,
                "FactCheck",
                "Verified truths, contradictions, and evidence.",
            );
            frame.add_slice(fact_id.clone(), slice);
        }

        fact_id
    }

    /// Run a basic fact-check pass:
    /// - scan Core and Delta slices
    /// - emit simple "seen" verdicts into FactCheck.
    pub fn run(&self, frame: &mut CognitiveFrame) -> Result<(), CognitiveFrameError> {
        let fact_slice_id = Self::ensure_factcheck_slice(frame);

        // Collect core + delta slices.
        let core_and_delta: Vec<(&SliceId, &Slice)> = frame
            .slices
            .iter()
            .filter(|(_, s)| matches!(s.metadata.slice_type, SliceType::Core | SliceType::Delta))
            .collect();

        for (sid, slice) in core_and_delta {
            for (cell_id, cell) in slice.cells_iter() {
                // For now, we just emit a "seen" fact-check entry.
                let verdict_cell_id = CellId::new_factcheck_for(cell_id);
                let verdict_cell = Cell {
                    id: verdict_cell_id.clone(),
                    cell_type: CellType::Fact,
                    content: CellContent::Text(format!(
                        "FactCheck: observed cell {:?} in slice {:?}",
                        cell_id, sid
                    )),
                    metadata: CellMetadata {
                        confidence: cell.metadata.confidence,
                        timestamp: Utc::now().timestamp(),
                        source: CellSource::Model,
                        tags: vec!["factcheck".into()],
                    },
                };

                frame.ingest_cell(&fact_slice_id, verdict_cell_id, verdict_cell)?;
            }
        }

        Ok(())
    }
}






