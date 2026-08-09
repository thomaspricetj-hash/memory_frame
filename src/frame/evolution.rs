use chrono::{DateTime, Utc};
use crate::frame::{MemoryFrame, CellId};
use crate::config::MemoryPolicy;

/// Event describing a change to a cell's confidence.
#[derive(Debug, Clone)]
pub struct EvolutionEvent {
    pub cell: CellId,
    pub timestamp: DateTime<Utc>,
    pub delta_confidence: f32,
}

impl MemoryFrame {
    /// Apply exponential decay to all cells' confidence values.
    ///
    /// Uses each cell's `last_updated` timestamp when available; otherwise treats age as zero.
    /// `half_life_secs` is taken from the frame policy and must be > 0.0 to have effect.
    pub fn apply_decay(&mut self, now: DateTime<Utc>) {
        let policy: &MemoryPolicy = &self.policy;

        // Guard against invalid half-life values.
        let half_life_secs = (policy.decay.time_half_life_secs as f64).max(1.0);
        let floor = policy.decay.confidence_floor;

        // Precompute reciprocal to avoid repeated division.
        let inv_half = 1.0 / half_life_secs;

        for slice in self.slices.values_mut() {
            for cell in &mut slice.grid.cells {
                // Determine age in seconds using the cell's last_updated if present.
                let age_secs = match cell.last_updated {
                    Some(ts) => {
                        let age = now.signed_duration_since(ts).num_seconds();
                        if age < 0 { 0 } else { age }
                    }
                    None => 0,
                } as f64;

                // Exponential decay factor: exp(-age / half_life)
                let factor = (-age_secs * inv_half).exp() as f32;

                let old_conf = cell.confidence;
                let new_conf = (old_conf * factor).max(floor);

                // Apply decay
                cell.confidence = new_conf;

                // Update timestamp to reflect decay event
                cell.last_updated = Some(now);

                // Optional: record evolution event (future use)
                // self.events.push(EvolutionEvent {
                //     cell: cell.id,
                //     timestamp: now,
                //     delta_confidence: new_conf - old_conf,
                // });
            }
        }
    }

    /// Return a reference to the frame's memory policy.
    pub fn policy(&self) -> &MemoryPolicy {
        &self.policy
    }
}

