// src/frame/cell_types.rs
//
// Polymath cell definitions for the hybrid cognitive memory engine.
// These types unify neural content (embeddings, RAG chunks) with symbolic
// content (rules, graph nodes, gestures) inside a single Cell abstraction.
//
// This file does NOT define the Cell struct itself â€” your existing
// src/frame/cell.rs owns that. Instead, this file defines the enums and
// metadata that Cell will use.

use serde::{Serialize, Deserialize};

/// The cognitive role of a cell inside a slice.
///
/// These roles determine how the cell participates in reasoning,
/// delta anchoring, fact-checking, semantic routing, and gesture mapping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellType {
    /// Canonical fact or stable truth.
    Fact,

    /// Difference, correction, update, or version delta.
    Delta,

    /// Emotional tone, vibe, or affect.
    Emotion,

    /// Drift-control rule, stability rule, semantic alignment rule.
    Rule,

    /// Gesture, movement, orientation, facial expression.
    Gesture,

    /// Concept, theme, embedding, semantic cluster.
    Semantic,

    /// Knowledge graph node or edge reference.
    GraphNode,
}

/// The actual content stored inside a cell.
///
/// This is intentionally flexible â€” your engine can store neural,
/// symbolic, or multi-modal data in the same structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellContent {
    /// Plain text, RAG chunk, or structured string data.
    Text(String),

    /// Embedding vector (neural semantic representation).
    Embedding(Vec<f32>),

    /// Knowledge graph node or edge identifier.
    GraphRef {
        node_id: String,
        relation: Option<String>,
    },

    /// Gesture representation for sign-language and spatial reasoning.
    Gesture {
        handshape: String,
        movement: String,
        orientation: String,
        location: String,
        facial: Option<String>,
    },

    /// Structured rule definition (symbolic).
    Rule {
        name: String,
        predicate: String,
        threshold: Option<f32>,
    },

    /// Arbitrary JSON payload for flexible future extensions.
    Json(serde_json::Value),
}

/// Metadata attached to every cell.
///
/// This is used for confidence scoring, temporal reasoning,
/// provenance tracking, and cross-reference linking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMetadata {
    /// Confidence score (0.0â€“1.0).
    pub confidence: f32,

    /// Timestamp when the cell was created.
    pub timestamp: i64,

    /// Source of the cell (user, model, external).
    pub source: CellSource,

    /// Optional tags for diagnostics, routing, or visualization.
    pub tags: Vec<String>,
}

/// Provenance of a cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellSource {
    User,
    Model,
    External,
}

impl CellMetadata {
    pub fn new(source: CellSource) -> Self {
        Self {
            confidence: 1.0,
            timestamp: chrono::Utc::now().timestamp(),
            source,
            tags: Vec::new(),
        }
    }

    pub fn with_confidence(source: CellSource, confidence: f32) -> Self {
        Self {
            confidence,
            timestamp: chrono::Utc::now().timestamp(),
            source,
            tags: Vec::new(),
        }
    }
}






