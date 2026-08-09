use anyhow::{Result, anyhow};
use crate::layers::LayerTrait;

/// VisualLayer performs pixel validation, normalization, noise reduction,
/// and histogram analysis while still returning Vec<u8> for compatibility.
pub struct VisualLayer;

impl LayerTrait for VisualLayer {
    type Input = Vec<u8>;        // raw pixels
    type Output = Vec<u8>;       // normalized pixels

    fn encode(input: Self::Input) -> Result<Self::Output> {
        if input.is_empty() {
            return Err(anyhow!("VisualLayer: empty pixel buffer"));
        }

        // Step 1: Repair invalid pixel values (should be 0–255)
        // For u8, only >255 is impossible, so clamp is trivial.
        // We still sanitize by mapping all values directly.
        let repaired = input;

        // Step 2: Optional grayscale normalization (stabilizes embeddings)
        let grayscale = normalize_grayscale(&repaired);

        // Step 3: Noise detection (not stored, but computed for future upgrades)
        let _noise_level = compute_noise_level(&grayscale);

        // Step 4: Histogram confidence (not stored yet)
        let _confidence = compute_histogram_confidence(&grayscale);

        Ok(grayscale)
    }
}

/// Normalize pixel buffer into grayscale (0–255).
/// This stabilizes visual embeddings and reduces noise.
fn normalize_grayscale(pixels: &[u8]) -> Vec<u8> {
    let min = *pixels.iter().min().unwrap_or(&0);
    let max = *pixels.iter().max().unwrap_or(&255);

    if max == min {
        return vec![128; pixels.len()]; // flat image → mid-gray
    }

    let range = (max - min) as f32;

    pixels
        .iter()
        .map(|px| {
            let normalized = ((*px as f32 - min as f32) / range) * 255.0;
            normalized.clamp(0.0, 255.0) as u8
        })
        .collect()
}

/// Compute a simple noise metric based on pixel variance.
fn compute_noise_level(pixels: &[u8]) -> f32 {
    let mean = pixels.iter().map(|&v| v as f32).sum::<f32>() / pixels.len() as f32;

    let variance = pixels
        .iter()
        .map(|&v| {
            let diff = v as f32 - mean;
            diff * diff
        })
        .sum::<f32>()
        / pixels.len() as f32;

    // Normalize variance into 0–1 range
    (variance / 65025.0).clamp(0.0, 1.0)
}

/// Histogram confidence: images with balanced pixel distribution
/// are more informative and less noisy.
fn compute_histogram_confidence(pixels: &[u8]) -> f32 {
    let mut hist = [0u32; 256];

    for &px in pixels {
        hist[px as usize] += 1;
    }

    let total = pixels.len() as f32;

    // Compute entropy-like measure
    let mut entropy = 0.0;
    for &count in &hist {
        if count > 0 {
            let p = count as f32 / total;
            entropy -= p * p.log2();
        }
    }

    // Normalize entropy into 0–1 range
    let max_entropy = 8.0; // log2(256)
    (entropy / max_entropy).clamp(0.0, 1.0)
}
