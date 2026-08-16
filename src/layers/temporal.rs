use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, Datelike};   // <-- FIX: import Datelike
use crate::layers::LayerTrait;

/// TemporalLayer enriches timestamps with validation, normalization,
/// and recency scoring while still returning a DateTime<Utc> for compatibility.
pub struct TemporalLayer;

impl LayerTrait for TemporalLayer {
    type Input = DateTime<Utc>;
    type Output = DateTime<Utc>;

    fn encode(input: Self::Input) -> Result<Self::Output> {
        // Reject nonsensical timestamps (year 0, far future, etc.)
        if input.year() < 1900 {
            return Err(anyhow!("TemporalLayer: timestamp too early"));
        }
        if input.year() > 3000 {
            return Err(anyhow!("TemporalLayer: timestamp too far in the future"));
        }

        // Normalize to UTC (already UTC, but ensures consistency)
        let normalized = input.with_timezone(&Utc);

        // Compute recency score (not stored yet)
        let _recency = compute_recency(normalized);

        // Compute temporal confidence (not stored yet)
        let _confidence = compute_confidence(normalized);

        Ok(normalized)
    }
}

/// Recency score: 1.0 = now, 0.0 = very old.
fn compute_recency(ts: DateTime<Utc>) -> f32 {
    let now = Utc::now();
    let delta = now.signed_duration_since(ts);

    if delta.num_seconds() < 0 {
        return 0.0;
    }

    let seconds = delta.num_seconds() as f32;
    let recency = (-seconds / (30.0 * 24.0 * 3600.0)).exp();

    recency.clamp(0.0, 1.0)
}

/// Confidence score: timestamps close to "now" are more trustworthy.
fn compute_confidence(ts: DateTime<Utc>) -> f32 {
    let now = Utc::now();
    let delta = now.signed_duration_since(ts);

    if delta.num_seconds() < 0 {
        return 0.1;
    }

    let seconds = delta.num_seconds() as f32;
    let confidence = (-seconds / (365.0 * 24.0 * 3600.0)).exp();

    confidence.clamp(0.0, 1.0)
}






