// src/neural/rag.rs
//
// RAG ingestion utilities for the hybrid cognitive memory engine.
// Converts retrieved text chunks into semantic cells and inserts them
// into the Semantic slice of a CognitiveFrame.

use chrono::Utc;

use crate::frame::cognitive_frame::{CognitiveFrame, CognitiveFrameError};
use crate::frame::slice_types::{SliceType, SliceMetadata};
use crate::frame::cell_types::{CellType, CellContent, CellMetadata, CellSource};
use crate::frame::{SliceId, CellId, Slice, Cell};

/// A simple RAG chunk structure.
/// You can expand this later with scores, sources, metadata, etc.
#[derive(Debug, Clone)]
pub struct RagChunk {
    pub text: String,
    pub score: f32,
    pub source: String,
}

/// RAG ingestion engine.
///
/// Responsibilities:
/// - ensure Semantic slice exists
/// - convert RAG chunks into semantic cells
/// - insert them into the Semantic slice
pub struct RagEngine;

impl RagEngine {
    pub fn new() -> Self {
        RagEngine
    }

    /// Ensure a Semantic slice exists; create if missing.
    fn ensure_semantic_slice(frame: &mut CognitiveFrame) -> SliceId {
        let sem_id = SliceId::new_semantic(); // you will add this helper.

        if frame.slice(&sem_id).is_none() {
            let mut slice = Slice::new(sem_id.clone());
            slice.metadata = SliceMetadata::described(
                SliceType::Semantic,
                "Semantic",
                "Concepts, themes, embeddings, semantic clusters.",
            );
            frame.add_slice(sem_id.clone(), slice);
        }

        sem_id
    }

    /// Ingest a batch of RAG chunks into the Semantic slice.
    pub fn ingest_chunks(
        &self,
        frame: &mut CognitiveFrame,
        chunks: Vec<RagChunk>,
    ) -> Result<(), CognitiveFrameError> {
        let sem_slice_id = Self::ensure_semantic_slice(frame);

        for chunk in chunks {
            let cell_id = CellId::new_semantic_cell();

            let cell = Cell {
                id: cell_id.clone(),
                cell_type: CellType::Semantic,
                content: CellContent::Text(chunk.text.clone()),
                metadata: CellMetadata {
                    confidence: chunk.average_confidence(),
                    timestamp: Utc::now().timestamp(),
                    source: CellSource::External,
                    tags: vec![format!("rag:{}", chunk.source)],
                },
            };

            frame.ingest_cell(&sem_slice_id, cell_id, cell)?;
        }

        Ok(())
    }
}






