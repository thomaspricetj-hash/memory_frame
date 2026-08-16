use anyhow::{Result, anyhow};
use chrono::{Utc, DateTime};
use crate::layers::LayerTrait;

/// Declarative knowledge is factual, stable, and structured.
/// This upgraded encoder normalizes, validates, and annotates the input.
pub struct DeclarativeLayer;

#[derive(Debug, Clone)]
pub struct DeclarativeOutput {
    pub text: String,
    pub normalized: String,
    pub timestamp: DateTime<Utc>,
    pub confidence: f32,
}

impl LayerTrait for DeclarativeLayer {
    type Input = String;
    type Output = DeclarativeOutput;

    fn encode(input: Self::Input) -> Result<Self::Output> {
        if input.trim().is_empty() {
            return Err(anyhow!("DeclarativeLayer: empty declarative input"));
        }

        let normalized = normalize_declarative(&input);
        let confidence = compute_confidence(&normalized);

        Ok(DeclarativeOutput {
            text: input,
            normalized,
            timestamp: Utc::now(),
            confidence,
        })
    }
}

/// Normalize declarative text into a stable canonical form.
fn normalize_declarative(s: &str) -> String {
    let mut out = s.trim().to_string();

    while out.contains("  ") {
        out = out.replace("  ", " ");
    }

    out = out.replace(" ,", ",");
    out = out.replace(" .", ".");
    out = out.replace(" !", "!");
    out = out.replace(" ?", "?");

    out
}

/// Compute a simple confidence score based on structure.
fn compute_confidence(s: &str) -> f32 {
    let len = s.len() as f32;
    let base = (len / 200.0).min(1.0);

    let punctuation_bonus = if s.contains('.') || s.contains(',') {
        0.1
    } else {
        0.0
    };

    (base + punctuation_bonus).min(1.0)
}







