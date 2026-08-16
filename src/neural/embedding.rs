// src/neural/embedding.rs
//
// Neural embedding utilities for the hybrid cognitive memory engine.
// This module provides helpers for creating, normalizing, and comparing
// embedding vectors stored inside CellContent::Embedding.

use serde::{Serialize, Deserialize};

/// A simple wrapper around an embedding vector.
///
/// This keeps your neural layer clean and lets you expand later
/// (dimensionality checks, normalization, cosine similarity, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub values: Vec<f32>,
}

impl Embedding {
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Normalize the embedding to unit length.
    pub fn normalize(&mut self) {
        let norm = self
            .values
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();

        if norm > 0.0 {
            for v in &mut self.values {
                *v /= norm;
            }
        }
    }

    /// Compute cosine similarity between two embeddings.
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        let dot = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum::<f32>();

        let norm_a = self
            .values
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();

        let norm_b = other
            .values
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}






