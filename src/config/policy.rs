use crate::config::types::*;
use crate::config::defaults::default_policy;
use serde::{Serialize, Deserialize};

/// Central controller for all runtime memory policies.
/// Supports loading, validating, updating, and hotâ€‘reloading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyManager {
    pub active: MemoryPolicy,
}

impl PolicyManager {
    /// Create a new manager with the default policy.
    pub fn new() -> Self {
        Self {
            active: default_policy(),
        }
    }

    /// Load a policy from JSON text.
    /// Performs validation automatically.
    pub fn load_from_json(json: &str) -> anyhow::Result<Self> {
        let policy: MemoryPolicy = serde_json::from_str(json)?;
        let manager = Self { active: policy };
        manager.validate()?;     // autoâ€‘validate
        Ok(manager)
    }

    /// Replace the active policy at runtime.
    /// Useful for hotâ€‘reload or adaptive tuning.
    pub fn set_policy(&mut self, policy: MemoryPolicy) -> anyhow::Result<()> {
        let manager = PolicyManager { active: policy };
        manager.validate()?;     // ensure new policy is safe
        self.active = manager.active;
        Ok(())
    }

    /// Export the active policy as JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(&self.active)?)
    }

    /// Validate all policy fields.
    /// Ensures no invalid or dangerous configuration enters the engine.
    pub fn validate(&self) -> anyhow::Result<()> {
        let p = &self.active;

        // -------------------------
        // Decay policy validation
        // -------------------------
        if p.decay.time_half_life_secs == 0 {
            anyhow::bail!("time_half_life_secs cannot be zero");
        }

        if !(0.0..=1.0).contains(&p.decay.confidence_floor) {
            anyhow::bail!("confidence_floor must be between 0 and 1");
        }

        // -------------------------
        // Conflict policy validation
        // -------------------------
        if !p.conflict.prefer_newer && !p.conflict.prefer_higher_confidence {
            anyhow::bail!("At least one conflict preference must be enabled");
        }

        // -------------------------
        // Compression policy validation
        // -------------------------
        if p.compression.cell_merge_threshold < 0.0
            || p.compression.cell_merge_threshold > 1.0
        {
            anyhow::bail!("cell_merge_threshold must be between 0 and 1");
        }

        if p.compression.max_redundant_cells == 0 {
            anyhow::bail!("max_redundant_cells cannot be zero");
        }

        // -------------------------
        // Crossâ€‘connect policy validation
        // -------------------------
        if p.cross_connect.max_links_per_cell == 0 {
            anyhow::bail!("max_links_per_cell cannot be zero");
        }

        if p.cross_connect.default_weight <= 0.0 {
            anyhow::bail!("default_weight must be positive");
        }

        if p.cross_connect.decay_per_hop < 0.0 || p.cross_connect.decay_per_hop > 1.0 {
            anyhow::bail!("decay_per_hop must be between 0 and 1");
        }

        Ok(())
    }

    /// Convenience accessor.
    pub fn policy(&self) -> &MemoryPolicy {
        &self.active
    }
}






