// src/neural/attention.rs
//
// Short-term working memory (attention window) for the hybrid cognitive engine.
// This module provides helpers for inserting, refreshing, and decaying
// temporary cells inside the ShortTerm slice.
//
// The ShortTerm slice acts like an attention buffer:
// - fast inserts
// - fast lookups
// - automatic decay
// - no long-term guarantees

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// Configuration for short-term memory behavior.
#[derive(Debug, Clone)]
pub struct AttentionConfig {
    /// Maximum number of cells allowed in the attention window.
    pub max_cells: usize,

    /// Minimum confidence required to keep a cell during decay.
    pub min_confidence: f32,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            max_cells: 64,
            min_confidence: 0.15,
        }
    }
}

/// Short-term memory engine.
///
/// Responsibilities:
/// - ensure ShortTerm slice exists
/// - insert temporary cells
/// - refresh cells (boost confidence)
/// - decay cells (drop low-confidence entries)
pub struct AttentionEngine {
    pub config: AttentionConfig,
}

impl AttentionEngine {
    pub fn new(config: AttentionConfig) -> Self {
        Self { config }
    }

    /// Ensure a ShortTerm slice exists; create if missing.
    fn ensure_shortterm_slice(frame: &mut CognitiveFrame) -> SliceId {
        let st_id = SliceId::new_shortterm(); // you will add this helper.

        if frame.slice(&st_id).is_none() {
            let mut slice = Slice::new(st_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::ShortTerm,
                "ShortTerm",
                "Working memory / attention window.",
            );
            frame.add_slice(st_id.clone(), slice);
        }

        st_id
    }

    /// Insert a temporary cell into the attention window.
    pub fn insert_temp(
        &self,
        frame: &mut CognitiveFrame,
        content: CellContent,
        confidence: f32,
    ) -> Result<CellId, CognitiveFrameError> {
        let st_slice_id = Self::ensure_shortterm_slice(frame);

        let cell_id = CellId::new_shortterm_cell();

        let cell = Cell {
            id: cell_id.clone(),
            cell_type: CellType::Semantic, // short-term memory stores semantic-like content
            content,
            metadata: CellMetadata {
                confidence,
                timestamp: Utc::now().timestamp(),
                source: CellSource::Model,
                tags: vec!["shortterm".into()],
            },
        };

        frame.ingest_cell(&st_slice_id, cell_id.clone(), cell)?;
        self.enforce_capacity(frame, &st_slice_id)?;

        Ok(cell_id)
    }

    /// Refresh a cell in the attention window (boost confidence).
    pub fn refresh(
        &self,
        frame: &mut CognitiveFrame,
        cell_id: &CellId,
        boost: f32,
    ) -> Result<(), CognitiveFrameError> {
        let st_slice_id = Self::ensure_shortterm_slice(frame);

        let slice = frame
            .slice_mut(&st_slice_id)
            .ok_or_else(|| CognitiveFrameError::SliceNotFound(st_slice_id.clone()))?;

        if let Some(cell) = slice.get_cell_mut(cell_id) {
            cell.metadata.confidence =
                (cell.metadata.confidence + boost).min(1.0);
        }

        Ok(())
    }

    /// Decay low-confidence cells.
    pub fn decay(&self, frame: &mut CognitiveFrame) -> Result<(), CognitiveFrameError> {
        let st_slice_id = Self::ensure_shortterm_slice(frame);

        let slice = frame
            .slice_mut(&st_slice_id)
            .ok_or_else(|| CognitiveFrameError::SliceNotFound(st_slice_id.clone()))?;

        let min_conf = self.config.min_confidence;

        slice.retain_cells(|cell| cell.metadata.confidence >= min_conf);

        Ok(())
    }

    /// Enforce maximum capacity by dropping oldest cells.
    fn enforce_capacity(
        &self,
        frame: &mut CognitiveFrame,
        st_slice_id: &SliceId,
    ) -> Result<(), CognitiveFrameError> {
        let slice = frame
            .slice_mut(st_slice_id)
            .ok_or_else(|| CognitiveFrameError::SliceNotFound(st_slice_id.clone()))?;

        let max = self.config.max_cells;

        if slice.len() > max {
            slice.drop_oldest(slice.len() - max);
        }

        Ok(())
    }
}






