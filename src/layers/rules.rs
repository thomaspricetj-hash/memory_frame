// src/layers/rules.rs
//
// Drift-control rule layer for the hybrid cognitive memory engine.
// Applies stability, confidence, and basic semantic alignment rules
// over slices and cells, and records rule activity in the Rule slice.

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// Simple drift-control rule engine.
///
/// This is a conservative starter implementation:
/// - scans Core + Delta slices
/// - emits rule activity into the Rule slice.
pub struct RuleEngine;

impl RuleEngine {
    pub fn new() -> Self {
        RuleEngine
    }

    /// Ensure a Rule slice exists; create if missing.
    fn ensure_rule_slice(frame: &mut CognitiveFrame) -> SliceId {
        let rule_id = SliceId::new_rule(); // you will add this helper.

        if frame.slice(&rule_id).is_none() {
            let mut slice = Slice::new(rule_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::Rule,
                "Rule",
                "Drift-control, stability, and semantic alignment rules.",
            );
            frame.add_slice(rule_id.clone(), slice);
        }

        rule_id
    }

    /// Run a basic rule pass:
    /// - scan Core + Delta
    /// - emit simple "rule applied" entries into Rule slice.
    pub fn run(&self, frame: &mut CognitiveFrame) -> Result<(), CognitiveFrameError> {
        let rule_slice_id = Self::ensure_rule_slice(frame);

        let core_and_delta: Vec<(&SliceId, &Slice)> = frame
            .slices
            .iter()
            .filter(|(_, s)| matches!(s.metadata.slice_type, SliceType::Core | SliceType::Delta))
            .collect();

        for (sid, slice) in core_and_delta {
            for (cell_id, cell) in slice.cells_iter() {
                let rule_cell_id = CellId::new_rule_for(cell_id);
                let rule_cell = Cell {
                    id: rule_cell_id.clone(),
                    cell_type: CellType::Rule,
                    content: CellContent::Text(format!(
                        "RuleEngine: evaluated cell {:?} in slice {:?}",
                        cell_id, sid
                    )),
                    metadata: CellMetadata {
                        confidence: cell.metadata.confidence,
                        timestamp: Utc::now().timestamp(),
                        source: CellSource::Model,
                        tags: vec!["rule".into()],
                    },
                };

                frame.ingest_cell(&rule_slice_id, rule_cell_id, rule_cell)?;
            }
        }

        Ok(())
    }
}






