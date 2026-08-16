// src/frame/slice_types.rs
//
// Canonical slice types for the hybrid cognitive memory engine.
// Every slice in a CognitiveFrame must declare one of these types.
//
// These types define the cognitive role of each slice and allow
// routing, rule enforcement, delta anchoring, fact-checking,
// cross-referencing, semantic processing, and gesture mapping.

use serde::{Serialize, Deserialize};

/// The cognitive role of a slice inside a CognitiveFrame.
///
/// This enum is intentionally stable â€” you will build engines,
/// rule systems, and cross-reference logic around these variants.
/// Avoid changing names once the system grows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SliceType {
    /// Canonical truth. Stable meaning. High-confidence facts.
    Core,

    /// Updates, corrections, differences, and version history.
    Delta,

    /// Verified truths, contradictions, external evidence.
    FactCheck,

    /// Links to other memories, timelines, topics, entities, gestures.
    CrossRef,

    /// Concepts, themes, embeddings, semantic clusters.
    Semantic,

    /// Tone, vibe, affect, emotional context.
    Emotional,

    /// Drift-control rules, stability rules, semantic alignment rules.
    Rule,

    /// Gesture, movement, orientation, facial expression, spatial indexing.
    SignLanguage,

    /// Short-term working memory (attention window).
    ShortTerm,
}

/// Metadata describing a slice inside a CognitiveFrame.
///
/// This is lightweight but extremely useful for diagnostics,
/// visualization, and routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceMetadata {
    pub slice_type: SliceType,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl SliceMetadata {
    pub fn new(slice_type: SliceType) -> Self {
        Self {
            slice_type,
            name: None,
            description: None,
        }
    }

    pub fn named(slice_type: SliceType, name: impl Into<String>) -> Self {
        Self {
            slice_type,
            name: Some(name.into()),
            description: None,
        }
    }

    pub fn described(
        slice_type: SliceType,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            slice_type,
            name: Some(name.into()),
            description: Some(description.into()),
        }
    }
}






