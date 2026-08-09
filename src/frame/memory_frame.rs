// src/frame/memory_frame.rs

use std::collections::HashMap;
use uuid::Uuid;

use crate::config::MemoryPolicy;
use crate::frame::{Slice, SliceId, SliceData, CrossConnect, CellId};
use crate::frame::adaptive_rules::{AdaptiveRuleEngine, AdaptiveScore};

#[derive(Debug)]
pub struct MemoryFrame {
    pub id: Uuid,
    pub slices: HashMap<SliceId, Slice>,
    pub cross_connect: CrossConnect,
    pub policy: MemoryPolicy,

    // 🔥 Diagonal frame-level propagation metrics
    pub diagonal_frame_signature: f32,
    pub diagonal_semantic_surface: f32,
    pub diagonal_confidence_surface: f32,
    pub diagonal_propagation_weight: f32,
    pub diagonal_collapse_influence: f32,

    // 🔥 Temporal diagonal law metrics
    pub temporal_alignment_score: f32,
    pub temporal_decay_factor: f32,
    pub temporal_diagonal_signature: f32,

    // 🔥 Adaptive rules / scoring (max-tier)
    pub adaptive_score: f32,
    pub last_adaptive: Option<AdaptiveScore>,
}

impl MemoryFrame {
    pub fn new(policy: MemoryPolicy) -> Self {
        Self {
            id: Uuid::new_v4(),
            slices: HashMap::new(),
            cross_connect: CrossConnect::new(),
            policy,

            diagonal_frame_signature: 0.0,
            diagonal_semantic_surface: 0.0,
            diagonal_confidence_surface: 0.0,
            diagonal_propagation_weight: 1.0,
            diagonal_collapse_influence: 1.0,

            temporal_alignment_score: 0.0,
            temporal_decay_factor: 1.0,
            temporal_diagonal_signature: 0.0,

            adaptive_score: 0.0,
            last_adaptive: None,
        }
    }

    /// Core auto-adaptive hook: recompute diagonal + temporal + adaptive rules.
    fn auto_adapt(&mut self) {
        // Recompute frame metrics first
        self.recompute_diagonal_metrics();
        self.recompute_temporal_metrics();

        // Clone policy locally to avoid simultaneous immutable borrow of self.policy
        // while also mutably borrowing self inside apply_to_frame.
        let policy_local = self.policy.clone();

        // Adaptive rules sit on top of those metrics.
        let mut engine = AdaptiveRuleEngine::default();
        let score = engine.apply_to_frame(self, &policy_local);

        self.adaptive_score = score.total;
        self.last_adaptive = Some(score);
    }

    pub fn insert_slice(&mut self, id: SliceId, data: SliceData) {
        let slice = Slice::new(id.clone(), data);
        self.slices.insert(id, slice);
        self.auto_adapt();
    }

    pub fn get_slice(&self, id: &SliceId) -> Option<&Slice> {
        self.slices.get(id)
    }

    /// Caller must call `touch_slice(id)` after mutation.
    pub fn get_slice_mut(&mut self, id: &SliceId) -> Option<&mut Slice> {
        self.slices.get_mut(id)
    }

    /// Recompute metrics after external mutation + adaptive rules.
    pub fn touch_slice(&mut self, id: &SliceId) {
        if self.slices.contains_key(id) {
            self.auto_adapt();
        }
    }

    pub fn global_confidence(&self) -> f32 {
        let mut total = 0.0;
        let mut count = 0;

        for slice in self.slices.values() {
            total += slice.average_confidence();
            count += 1;
        }

        if count == 0 { 0.0 } else { total / count as f32 }
    }

    pub fn list_slices(&self) -> Vec<SliceId> {
        self.slices.keys().cloned().collect()
    }

    pub fn has_slice(&self, id: &SliceId) -> bool {
        self.slices.contains_key(id)
    }

    pub fn remove_slice(&mut self, id: &SliceId) -> Option<Slice> {
        let out = self.slices.remove(id);
        if out.is_some() {
            self.auto_adapt();
        }
        out
    }

    pub fn total_cell_count(&self) -> usize {
        self.slices.values().map(|s| s.grid.cell_count()).sum()
    }

    pub fn find_slice_for_cell(&self, cell: CellId) -> Option<SliceId> {
        self.slices
            .iter()
            .find(|(_, slice)| slice.grid.get_cell(cell).is_some())
            .map(|(sid, _)| sid.clone())
    }

    pub fn slices_with_tag(&self, tag: &str) -> Vec<SliceId> {
        self.slices
            .iter()
            .filter(|(_, slice)| slice.grid.cells.iter().any(|c| c.tags.contains(&tag.to_string())))
            .map(|(sid, _)| sid.clone())
            .collect()
    }

    // -------------------------------------------------------------------------
    // 🔥 DIAGONAL FRAME-LEVEL PROPAGATION (MAX TIER)
    // -------------------------------------------------------------------------

    pub fn recompute_diagonal_metrics(&mut self) {
        if self.slices.is_empty() {
            self.diagonal_frame_signature = 0.0;
            self.diagonal_semantic_surface = 0.0;
            self.diagonal_confidence_surface = 0.0;
            self.diagonal_propagation_weight = 1.0;
            self.diagonal_collapse_influence = 1.0;
            return;
        }

        let mut sig_total = 0.0;
        let mut sem_total = 0.0;
        let mut conf_total = 0.0;
        let mut collapse_total = 0.0;
        let mut count = 0.0;

        for slice in self.slices.values() {
            sig_total += slice.diagonal_signature();
            sem_total += slice.diagonal_semantic_surface();
            conf_total += slice.diagonal_confidence_surface();
            collapse_total += slice.diagonal_collapse_influence();
            count += 1.0;
        }

        self.diagonal_frame_signature = sig_total / count;
        self.diagonal_semantic_surface = sem_total / count;
        self.diagonal_confidence_surface = conf_total / count;

        self.diagonal_propagation_weight =
            ((self.diagonal_semantic_surface + self.diagonal_confidence_surface) / 2.0)
                .clamp(0.25, 2.0);

        self.diagonal_collapse_influence =
            (collapse_total / count).clamp(0.25, 2.0);
    }

    // -------------------------------------------------------------------------
    // 🔥 TEMPORAL DIAGONAL LAW (FRAME-LEVEL)
    // -------------------------------------------------------------------------

    pub fn recompute_temporal_metrics(&mut self) {
        use chrono::{Utc, Duration};

        if self.slices.is_empty() {
            self.temporal_alignment_score = 0.0;
            self.temporal_decay_factor = 1.0;
            self.temporal_diagonal_signature = 0.0;
            return;
        }

        let now = Utc::now();

        let mut aligned_sum = 0.0;
        let mut decay_sum = 0.0;
        let mut diag_temporal_sum = 0.0;
        let mut count = 0.0;

        for slice in self.slices.values() {
            let mut slice_alignment = 0.0;
            let mut slice_decay = 0.0;
            let mut slice_count = 0.0;

            for cell in &slice.grid.cells {
                if let Some(ts) = cell.last_updated {
                    let age: Duration = now.signed_duration_since(ts);
                    let secs = age.num_seconds().max(0) as f32;

                    // Temporal decay: older cells contribute less
                    let decay = (1.0 / (1.0 + secs / 10.0)).clamp(0.0, 1.0);

                    // Alignment: recent + confident cells are more aligned
                    let align = (cell.confidence * decay).clamp(0.0, 1.5);

                    slice_alignment += align;
                    slice_decay += decay;
                    slice_count += 1.0;
                }
            }

            if slice_count > 0.0 {
                let avg_align = slice_alignment / slice_count;
                let avg_decay = slice_decay / slice_count;

                aligned_sum += avg_align;
                decay_sum += avg_decay;

                // Temporal diagonal signature: blend diagonal + temporal
                let diag = slice.diagonal_signature();
                diag_temporal_sum += diag * avg_decay;
                count += 1.0;
            }
        }

        if count == 0.0 {
            self.temporal_alignment_score = 0.0;
            self.temporal_decay_factor = 1.0;
            self.temporal_diagonal_signature = 0.0;
            return;
        }

        self.temporal_alignment_score = (aligned_sum / count).clamp(0.0, 1.5);
        self.temporal_decay_factor = (decay_sum / count).clamp(0.25, 1.0);
        self.temporal_diagonal_signature = (diag_temporal_sum / count).clamp(0.0, 2.0);
    }

    // -------------------------------------------------------------------------
    // Compacting / merging slices using harmonic signature + phase coherence
    // -------------------------------------------------------------------------

    /// Compact slices by merging semantically/harmonically similar slices.
    /// This is a conservative merge: it only merges when harmonic signature distance
    /// is small and phase coherence is above a threshold.
    pub fn compact_slices(&mut self) {
        // conservative defaults
        const PHASE_COHERENCE_THRESHOLD: f32 = 0.05;
        const SIGNATURE_DISTANCE_THRESHOLD: u64 = 0x00FF_FFFF; // heuristic

        // collect keys to iterate deterministically
        let keys: Vec<SliceId> = self.slices.keys().cloned().collect();

        for sid in keys {
            // if slice was removed/merged already, skip
            if !self.slices.contains_key(&sid) {
                continue;
            }

            // compute signature for candidate slice
            let sig = {
                if let Some(slice) = self.slices.get(&sid) {
                    slice.harmonic_signature()
                } else {
                    continue;
                }
            };

            // find best matching other slice (excluding itself)
            let mut best_match: Option<(SliceId, u64)> = None;

            for (other_id, other_slice) in self.slices.iter() {
                if *other_id == sid {
                    continue;
                }
                let other_sig = other_slice.harmonic_signature();
                let dist = sig ^ other_sig;
                let dist_abs = dist; // u64 xor used as distance proxy

                if dist_abs <= SIGNATURE_DISTANCE_THRESHOLD {
                    match &best_match {
                        None => best_match = Some((other_id.clone(), dist_abs)),
                        Some((_, best_dist)) => {
                            if dist_abs < *best_dist {
                                best_match = Some((other_id.clone(), dist_abs));
                            }
                        }
                    }
                }
            }

            if let Some((match_id, _dist)) = best_match {
                // re-check both slices exist
                // Clone candidate slice first to avoid simultaneous mutable/immutable borrows
                if let Some(candidate_clone) = self.slices.get(&sid).cloned() {
                    if let Some(base_slice) = self.slices.get_mut(&match_id) {
                        // compute phase coherence for candidate
                        let coherence = crate::frame::phase::phase_coherence(&candidate_clone.grid);

                        if coherence >= PHASE_COHERENCE_THRESHOLD {
                            // perform conservative cell-wise phase-aware merge:
                            // iterate cells by index and apply phase merge into base
                            let len = base_slice.grid.cells.len().min(candidate_clone.grid.cells.len());
                            for i in 0..len {
                                let base_cell = &mut base_slice.grid.cells[i];
                                let cand_cell = &candidate_clone.grid.cells[i];
                                base_cell.apply_phase_merge(cand_cell);
                            }

                            // merge slice-level metadata conservatively
                            // prefer base metadata; if none, take candidate's
                            if base_slice.data == SliceData::Declarative(String::new()) {
                                // no-op; keep existing
                            }

                            // remove candidate slice (sid) from map
                            self.slices.remove(&sid);
                        }
                    }
                }
            }
        }

        // after compaction, recompute frame metrics and adaptive rules
        self.auto_adapt();
    }
}





