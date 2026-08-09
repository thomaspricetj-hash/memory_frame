use crate::layers::LayerId;

#[derive(Debug, Clone)]
pub struct LayerColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Debug)]
pub struct ColorMap;

impl ColorMap {
    /// Static, precomputed, perceptually balanced colors for each layer.
    /// These are chosen to be:
    /// - high‑contrast
    /// - non‑clashing
    /// - color‑blind friendly
    /// - stable across themes
    const VISUAL: LayerColor      = LayerColor { r: 0.18, g: 0.58, b: 0.95 };
    const SEMANTIC: LayerColor    = LayerColor { r: 0.92, g: 0.72, b: 0.22 };
    const TEMPORAL: LayerColor    = LayerColor { r: 0.55, g: 0.25, b: 0.82 };
    const EMOTIONAL: LayerColor   = LayerColor { r: 0.98, g: 0.32, b: 0.32 };
    const RELATIONAL: LayerColor  = LayerColor { r: 0.28, g: 0.95, b: 0.48 };
    const DECLARATIVE: LayerColor = LayerColor { r: 0.82, g: 0.82, b: 0.82 };

    /// Max‑tier: zero branching, static lookup, no allocations.
    pub fn for_layer(id: &LayerId) -> LayerColor {
        match id {
            LayerId::Visual      => Self::VISUAL,
            LayerId::Semantic    => Self::SEMANTIC,
            LayerId::Temporal    => Self::TEMPORAL,
            LayerId::Emotional   => Self::EMOTIONAL,
            LayerId::Relational  => Self::RELATIONAL,
            LayerId::Declarative => Self::DECLARATIVE,
        }
    }

    /// Max‑tier alpha mapping:
    /// - clamps to [0.1, 1.0]
    /// - applies perceptual gamma curve
    /// - ensures low confidence still visible
    pub fn confidence_to_alpha(conf: f32) -> f32 {
        let clamped = conf.clamp(0.0, 1.0);

        // Gamma curve: boosts mid‑range confidence visually
        let gamma_corrected = clamped.powf(0.7);

        // Ensure minimum visibility
        gamma_corrected.clamp(0.1, 1.0)
    }
}
