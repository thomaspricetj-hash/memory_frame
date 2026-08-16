use anyhow::Result;

/// Maxâ€‘tier layer trait:
/// - preserves existing encode() signature
/// - adds optional metadata hooks
/// - adds validation helpers
/// - adds normalization helpers
/// - adds layer identity
/// - zero breakage for existing layers
pub trait LayerTrait {
    type Input;
    type Output;

    /// Core encoding function (unchanged for compatibility).
    fn encode(input: Self::Input) -> Result<Self::Output>;

    /// Optional: return the canonical name of the layer.
    /// Layers may override this, but they don't have to.
    fn layer_name() -> &'static str {
        "unknown"
    }

    /// Optional: validate input before encoding.
    /// Layers may override this to add preâ€‘checks.
    fn validate_input(_input: &Self::Input) -> Result<()> {
        Ok(())
    }

    /// Optional: normalize input before encoding.
    /// Layers may override this to canonicalize data.
    fn normalize_input(input: Self::Input) -> Self::Input {
        input
    }

    /// Full encode pipeline:
    /// - validate
    /// - normalize
    /// - encode
    ///
    /// Layers can use this automatically without overriding anything.
    fn encode_full(input: Self::Input) -> Result<Self::Output> {
        Self::validate_input(&input)?;
        let normalized = Self::normalize_input(input);
        Self::encode(normalized)
    }
}






