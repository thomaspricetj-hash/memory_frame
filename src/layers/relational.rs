use anyhow::{Result, anyhow};
use crate::layers::LayerTrait;

/// Relational embeddings benefit from normalization, stability checks,
/// and repair of invalid values. Output remains Vec<f32> for compatibility.
pub struct RelationalLayer;

impl LayerTrait for RelationalLayer {
    type Input = Vec<f32>;
    type Output = Vec<f32>;

    fn encode(input: Self::Input) -> Result<Self::Output> {
        if input.is_empty() {
            return Err(anyhow!("RelationalLayer: empty embedding vector"));
        }

        // Step 1: Repair invalid values (NaN, Inf)
        let mut repaired = input
            .into_iter()
            .map(|v| if v.is_finite() { v } else { 0.0 })
            .collect::<Vec<f32>>();

        // Step 2: L2 normalization for embedding stability
        let norm = repaired.iter().map(|v| v * v).sum::<f32>().sqrt();

        if norm > 0.0 {
            for v in &mut repaired {
                *v /= norm;
            }
        }

        // Step 3: Optional: clamp extreme values after normalization
        for v in &mut repaired {
            if *v > 1.0 {
                *v = 1.0;
            } else if *v < -1.0 {
                *v = -1.0;
            }
        }

        Ok(repaired)
    }
}
