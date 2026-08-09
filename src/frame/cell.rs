// src/frame/cell.rs
//


use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::{DateTime, Utc};

/// Coordinate identifier for a cell in a grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellId {
    pub x: usize,
    pub y: usize,
}

impl fmt::Display for CellId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Default for CellId {
    fn default() -> Self {
        CellId { x: 0, y: 0 }
    }
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// Unique coordinate identifier.
    pub id: CellId,

    /// Confidence score representing strength or relevance (legacy).
    pub confidence: f32,

 
    /// Initialize from `confidence` for backward compatibility when migrating old data.
    pub phase_pos: f32,
    pub phase_neg: f32,


    pub tags: Vec<String>,

    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_updated: Option<DateTime<Utc>>,

    /// Optional metadata payload for future expansion.
    pub metadata: Option<serde_json::Value>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            id: CellId::default(),
            confidence: 0.0,
            phase_pos: 0.0,
            phase_neg: 0.0,
            tags: Vec::new(),
            last_updated: None,
            metadata: None,
        }
    }
}

impl Cell {

    pub fn new(id: CellId) -> Self {
        Self {
            id,
            confidence: 0.0,
            phase_pos: 0.0,
            phase_neg: 0.0,
            tags: Vec::new(),
            last_updated: None,
            metadata: None,
        }
    }


    pub fn init_phases_from_confidence(&mut self) {
        if self.phase_pos == 0.0 && self.phase_neg == 0.0 {
            self.phase_pos = self.confidence;
            self.phase_neg = 0.0;
        }
    }


    pub fn touch(&mut self, new_confidence: f32) {
        self.confidence = new_confidence;
        self.last_updated = Some(Utc::now());
    }


    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }


    pub fn set_metadata(&mut self, value: serde_json::Value) {
        self.metadata = Some(value);
    }


    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    // -------------------------------------------------------------------------
    // Phase helpers (Russell-inspired dual polarity)
    // -------------------------------------------------------------------------


    pub fn phase_net(&self) -> f32 {
        self.phase_pos - self.phase_neg
    }


    pub fn phase_magnitude(&self) -> f32 {
        self.phase_net().abs()
    }


    pub fn apply_phase_merge(&mut self, other: &Cell) {
        // Keep the stronger phase channels (max) to preserve polarity extremes.
        self.phase_pos = self.phase_pos.max(other.phase_pos);
        self.phase_neg = self.phase_neg.max(other.phase_neg);

        // Blend confidence with a bias toward the higher confidence to avoid dilution.
        self.confidence = (self.confidence.max(other.confidence) * 0.7)
            + (self.confidence.min(other.confidence) * 0.3);

        // Merge tags deterministically (avoid duplicates).
        for t in &other.tags {
            if !self.tags.contains(t) {
                self.tags.push(t.clone());
            }
        }


        self.last_updated = Some(Utc::now());

        // Merge metadata conservatively: prefer existing, else take other's.
        if self.metadata.is_none() {
            self.metadata = other.metadata.clone();
        }
    }

    // -------------------------------------------------------------------------
    // 🔥 DIAGONAL LAW UPGRADES (NO NEW GRID METHODS REQUIRED)
    // -------------------------------------------------------------------------


    pub fn diagonal_semantic_boost(&self) -> f32 {
        let tag_count = self.tags.len() as f32;
        (1.0 + tag_count / 10.0).clamp(1.0, 1.5)
    }


    pub fn diagonal_confidence_influence(&self) -> f32 {
        (self.confidence * self.diagonal_semantic_boost()).clamp(0.0, 2.0)
    }


    pub fn diagonal_collapse_influence(&self) -> f32 {
        let base = self.confidence;
        let sem = self.diagonal_semantic_boost();
        (base * sem).clamp(0.1, 2.0)
    }


    pub fn diagonal_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        hasher.write_u64(self.confidence.to_bits() as u64);
        hasher.write_u64(self.phase_pos.to_bits() as u64);
        hasher.write_u64(self.phase_neg.to_bits() as u64);

        for t in &self.tags {
            t.hash(&mut hasher);
        }

        if let Some(meta) = &self.metadata {
            meta.to_string().hash(&mut hasher);
        }

        hasher.finish()
    }
}
