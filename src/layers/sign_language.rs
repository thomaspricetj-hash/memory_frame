// src/layers/sign_language.rs
//
// Sign-language / gesture layer for the hybrid cognitive memory engine.
// Converts gesture-like cells into structured gesture representations
// and links them into the SignLanguage slice.

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// Gesture processing engine.
///
/// For now, this just demonstrates the pattern:
/// - scan Gesture cells in any slice
/// - emit normalized gesture entries into SignLanguage slice.
pub struct SignLanguageEngine;

impl SignLanguageEngine {
    pub fn new() -> Self {
        SignLanguageEngine
    }

    /// Ensure a SignLanguage slice exists; create if missing.
    fn ensure_signlang_slice(frame: &mut CognitiveFrame) -> SliceId {
        let sign_id = SliceId::new_signlanguage(); // you will add this helper.

        if frame.slice(&sign_id).is_none() {
            let mut slice = Slice::new(sign_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::SignLanguage,
                "SignLanguage",
                "Gesture, movement, orientation, facial expression, spatial indexing.",
            );
            frame.add_slice(sign_id.clone(), slice);
        }

        sign_id
    }

    /// Run a basic gesture normalization pass:
    /// - scan all slices for Gesture cells
    /// - emit normalized gesture entries into SignLanguage slice.
    pub fn run(&self, frame: &mut CognitiveFrame) -> Result<(), CognitiveFrameError> {
        let sign_slice_id = Self::ensure_signlang_slice(frame);

        // Scan all slices for gesture cells.
        let all_slices: Vec<(&SliceId, &Slice)> = frame.slices.iter().collect();

        for (sid, slice) in all_slices {
            for (cell_id, cell) in slice.cells_iter() {
                if !matches!(cell.cell_type, CellType::Gesture) {
                    continue;
                }

                let gesture_cell_id = CellId::new_signlanguage_for(cell_id);

                let normalized_content = match &cell.content {
                    CellContent::Gesture {
                        handshape,
                        movement,
                        orientation,
                        location,
                        facial,
                    } => CellContent::Gesture {
                        handshape: handshape.clone(),
                        movement: movement.clone(),
                        orientation: orientation.clone(),
                        location: location.clone(),
                        facial: facial.clone(),
                    },
                    other => {
                        // Non-gesture content: just wrap as text for now.
                        CellContent::Text(format!(
                            "SignLanguage: non-gesture cell {:?} in slice {:?}: {:?}",
                            cell_id, sid, other
                        ))
                    }
                };

                let gesture_cell = Cell {
                    id: gesture_cell_id.clone(),
                    cell_type: CellType::Gesture,
                    content: normalized_content,
                    metadata: CellMetadata {
                        confidence: cell.metadata.confidence,
                        timestamp: Utc::now().timestamp(),
                        source: CellSource::Model,
                        tags: vec!["signlanguage".into()],
                    },
                };

                frame.ingest_cell(&sign_slice_id, gesture_cell_id, gesture_cell)?;
            }
        }

        Ok(())
    }
}






