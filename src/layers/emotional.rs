use anyhow::{Result, anyhow};
use chrono::{Utc, DateTime};
use crate::layers::LayerTrait;

/// Emotional signals are lightweight but benefit from stability,
/// normalization, and metadata for future reasoning.
pub struct EmotionalLayer;

#[derive(Debug, Clone)]
pub struct EmotionalOutput {
    pub value: f32,              // normalized 0–1 emotional intensity
    pub confidence: f32,         // stability score
    pub timestamp: DateTime<Utc> // when the signal was encoded
}

impl LayerTrait for EmotionalLayer {
    type Input = f32;
    type Output = EmotionalOutput;

    fn encode(input: Self::Input) -> Result<Self::Output> {
        if !input.is_finite() {
            return Err(anyhow!("EmotionalLayer: non‑finite emotional score"));
        }

        // Normalize into 0–1 range
        let value = input.clamp(0.0, 1.0);

        // Confidence heuristic: mid‑range values are more stable
        let confidence = compute_confidence(value);

        Ok(EmotionalOutput {
            value,
            confidence,
            timestamp: Utc::now(),
        })
    }
}

/// Emotional confidence curve:
/// - extreme values (0 or 1) are less stable
/// - mid‑range values (~0.5) are more stable
fn compute_confidence(v: f32) -> f32 {
    // Parabolic stability curve: peak at 0.5
    let stability = 1.0 - (v - 0.5).abs() * 2.0;
    stability.clamp(0.0, 1.0)
}
