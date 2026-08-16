// src/frame/memory_frame.rs

use std::collections::HashMap;
use uuid::Uuid;

use crate::config::MemoryPolicy;
use crate::frame::{Slice, SliceId, SliceData, CrossConnect, CellId};
use crate::frame::adaptive_rules::{AdaptiveRuleEngine, AdaptiveScore};
use crate::layers::LayerId;
use crate::frame::NavTarget;
use chrono::{DateTime, Utc};

/// Perception mode for view-dependent memory interpretation.
/// This does not change the underlying data, only how it is "seen" and weighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerceptionMode {
    /// Raw, structural view (existing behavior).
    Default,
    /// Emphasize semantic surfaces and tags.
    SemanticFocus,
    /// Emphasize temporal alignment and decay.
    TemporalFocus,
    /// Emphasize diagonal signatures and confidence surfaces.
    DiagonalFocus,
    /// Emphasize cross-connect topology.
    CrossConnectFocus,
}

/// Lightweight, frame-level perception transform.
/// Stores per-slice weights derived from the chosen mode.
#[derive(Debug, Clone)]
pub struct PerceptionTransform {
    pub mode: PerceptionMode,
    pub slice_weights: HashMap<SliceId, f32>,
}

impl PerceptionTransform {
    pub fn new(mode: PerceptionMode) -> Self {
        Self {
            mode,
            slice_weights: HashMap::new(),
        }
    }

    pub fn weight_for(&self, id: &SliceId) -> f32 {
        self.slice_weights.get(id).cloned().unwrap_or(1.0)
    }
}

#[derive(Debug)]
pub struct MemoryFrame {
    pub id: Uuid,
    pub slices: HashMap<SliceId, Slice>,
    pub slice_order: Vec<SliceId>, // preserve insertion order for deterministic navigation
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

    // 🔥 Perception: view-dependent weighting of slices
    pub perception_mode: PerceptionMode,
    pub perception_transform: PerceptionTransform,
}

impl MemoryFrame {
    pub fn new(policy: MemoryPolicy) -> Self {
        let mode = PerceptionMode::Default;
        let transform = PerceptionTransform::new(mode);

        Self {
            id: Uuid::new_v4(),
            slices: HashMap::new(),
            slice_order: Vec::new(),
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

            perception_mode: mode,
            perception_transform: transform,
        }
    }

    fn auto_adapt(&mut self) {
        self.recompute_diagonal_metrics();
        self.recompute_temporal_metrics();
        self.recompute_perception_transform();

        let policy_local = self.policy.clone();
        let mut engine = AdaptiveRuleEngine::default();
        let score = engine.apply_to_frame(self, &policy_local);

        self.adaptive_score = score.total;
        self.last_adaptive = Some(score);
    }

    pub fn insert_slice(&mut self, id: SliceId, data: SliceData) {
        // If the slice already exists, replace it but keep original insertion order.
        let exists = self.slices.contains_key(&id);
        let slice = Slice::new(id.clone(), data);
        self.slices.insert(id.clone(), slice);
        if !exists {
            self.slice_order.push(id.clone());
        }
        self.auto_adapt();
    }

    pub fn get_slice(&self, id: &SliceId) -> Option<&Slice> {
        self.slices.get(id)
    }

    pub fn get_slice_mut(&mut self, id: &SliceId) -> Option<&mut Slice> {
        self.slices.get_mut(id)
    }

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

    /// Return slice IDs in insertion order (deterministic).
    pub fn list_slices(&self) -> Vec<SliceId> {
        self.slice_order.clone()
    }

    pub fn has_slice(&self, id: &SliceId) -> bool {
        self.slices.contains_key(id)
    }

    pub fn remove_slice(&mut self, id: &SliceId) -> Option<Slice> {
        let out = self.slices.remove(id);
        if out.is_some() {
            // remove from insertion order vector
            if let Some(pos) = self.slice_order.iter().position(|x| x == id) {
                self.slice_order.remove(pos);
            }
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
    // 🔥 REQUIRED BY INTEGRATION TESTS
    // -------------------------------------------------------------------------

    /// Helper: return slices belonging to a layer.
    /// Preserves insertion order.
    pub fn slices_for_layer(&self, layer: LayerId) -> Vec<SliceId> {
        self.slice_order
            .iter()
            .filter(|sid| *sid == &layer)
            .cloned()
            .collect()
    }

    /// Helper: mutable slice iterator (returns IDs to avoid multiple simultaneous mutable borrows).
    /// Callers can then use `get_slice_mut(id)` to obtain a mutable reference when needed.
    pub fn slices_mut(&mut self) -> Vec<SliceId> {
        self.slice_order.clone()
    }

    /// REQUIRED: navigation API
    ///
    /// Interpretation:
    /// - If `current` is `Some(id)`, treat it as the *current slice id* and return
    ///   the neighbor slice according to insertion order:
    ///     - FirstSlice -> first inserted slice
    ///     - LastSlice  -> last inserted slice
    ///     - NextSlice  -> slice after `id` (if any)
    ///     - PrevSlice  -> slice before `id` (if any)
    /// - If `current` is `None`, operate on the global list (First/Last).
    pub fn navigate(
        &self,
        current: Option<LayerId>,
        target: NavTarget,
    ) -> Option<SliceId> {
        // Use insertion-ordered list
        let order = self.list_slices();

        if order.is_empty() {
            return None;
        }

        match current {
            None => {
                // No current context: First/Last only make sense here.
                match target {
                    NavTarget::FirstSlice => order.first().cloned(),
                    NavTarget::LastSlice => order.last().cloned(),
                    NavTarget::NextSlice => None,
                    NavTarget::PrevSlice => None,
                }
            }
            Some(cur_id) => {
                // Find index of current in insertion order
                let pos = order.iter().position(|id| *id == cur_id);

                match target {
                    NavTarget::FirstSlice => order.first().cloned(),
                    NavTarget::LastSlice => order.last().cloned(),
                    NavTarget::NextSlice => {
                        if let Some(idx) = pos {
                            if idx + 1 < order.len() {
                                Some(order[idx + 1].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    NavTarget::PrevSlice => {
                        if let Some(idx) = pos {
                            if idx >= 1 {
                                Some(order[idx - 1].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    /// Perception-aware navigation API.
    /// Uses the same semantics as `navigate`, but operates on the perceived slice order
    /// derived from the current `perception_mode`.
    pub fn navigate_perceived(
        &self,
        current: Option<LayerId>,
        target: NavTarget,
    ) -> Option<SliceId> {
        let order = self.perceived_slice_order();

        if order.is_empty() {
            return None;
        }

        match current {
            None => {
                match target {
                    NavTarget::FirstSlice => order.first().cloned(),
                    NavTarget::LastSlice => order.last().cloned(),
                    NavTarget::NextSlice => None,
                    NavTarget::PrevSlice => None,
                }
            }
            Some(cur_id) => {
                let pos = order.iter().position(|id| *id == cur_id);

                match target {
                    NavTarget::FirstSlice => order.first().cloned(),
                    NavTarget::LastSlice => order.last().cloned(),
                    NavTarget::NextSlice => {
                        if let Some(idx) = pos {
                            if idx + 1 < order.len() {
                                Some(order[idx + 1].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    NavTarget::PrevSlice => {
                        if let Some(idx) = pos {
                            if idx >= 1 {
                                Some(order[idx - 1].clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }

    /// REQUIRED: temporal decay API
    pub fn apply_decay(&mut self, _now: DateTime<Utc>) {
        for _slice in self.slices.values_mut() {
            /* frame-level decay: recompute_temporal_metrics() */
        }
        // Ensure frame-level temporal metrics are recomputed and adaptive rules applied.
        self.recompute_temporal_metrics();
        self.auto_adapt();
    }

    /// REQUIRED: conflict resolution API
    pub fn resolve_conflict(
        &mut self,
        a: SliceId,
        b: SliceId,
    ) -> Option<SliceId> {
        let sa = self.get_slice(&a)?;
        let sb = self.get_slice(&b)?;

        if sa.average_confidence() >= sb.average_confidence() {
            Some(a)
        } else {
            Some(b)
        }
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

                    let decay = (1.0 / (1.0 + secs / 10.0)).clamp(0.0, 1.0);
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

    pub fn compact_slices(&mut self) {
        const PHASE_COHERENCE_THRESHOLD: f32 = 0.05;
        const SIGNATURE_DISTANCE_THRESHOLD: u64 = 0x00FF_FFFF; // heuristic
        const INTRA_SLICE_MERGE_CONFIDENCE_THRESHOLD: f32 = 0.9; // conservative threshold for cell-level merges

        // First: perform intra-slice neighbor consolidation.
        // This addresses tests that expect neighboring cells within the same slice
        // to propagate tags/merge metadata (e.g., primary at (0,0) receiving tags from (1,0)).
        for slice in self.slices.values_mut() {
            // Build a map from coordinates to index for quick neighbor lookup.
            let mut coord_index: HashMap<(usize, usize), usize> = HashMap::new();
            for (idx, cell) in slice.grid.cells.iter().enumerate() {
                coord_index.insert((cell.id.x, cell.id.y), idx);
            }

            // Iterate over a snapshot of indices to avoid borrowing issues while mutating cells.
            let indices: Vec<usize> = (0..slice.grid.cells.len()).collect();
            for idx in indices {
                // ensure index still valid (slice may have changed length in other code paths)
                if idx >= slice.grid.cells.len() {
                    continue;
                }
                // current cell coordinates
                let (cx, cy) = {
                    let c = &slice.grid.cells[idx];
                    (c.id.x, c.id.y)
                };

                // neighbor to the right
                let neighbor_coord = (cx + 1, cy);
                if let Some(&nidx) = coord_index.get(&neighbor_coord) {
                    // ensure indices are still valid
                    if nidx >= slice.grid.cells.len() || idx >= slice.grid.cells.len() {
                        continue;
                    }

                    if idx == nidx {
                        continue;
                    }

                    // Ensure we always take the lower index as the "primary" target for deterministic behavior
                    let (primary_idx, neighbor_idx) =
                        if idx < nidx { (idx, nidx) } else { (nidx, idx) };

                    // At this point primary_idx < neighbor_idx guaranteed.
                    let (left, right) = slice.grid.cells.split_at_mut(neighbor_idx);
                    let primary_cell = &mut left[primary_idx];
                    let neighbor_cell = &mut right[0];

                    // Only merge when both confidences meet threshold (conservative)
                    if primary_cell.confidence >= INTRA_SLICE_MERGE_CONFIDENCE_THRESHOLD
                        && neighbor_cell.confidence >= INTRA_SLICE_MERGE_CONFIDENCE_THRESHOLD
                    {
                        // Propagate tags from neighbor into primary, dedupe
                        primary_cell.tags.extend(neighbor_cell.tags.iter().cloned());
                        primary_cell.tags.sort();
                        primary_cell.tags.dedup();

                        // Clear neighbor tags to indicate propagation (we keep the cell)
                        neighbor_cell.tags.clear();
                    }
                }
            }

            // Note: we intentionally do not remove cells here; we only propagate tags conservatively.
        }

        // Next: perform cross-slice compaction/merge as before.
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
                        let coherence =
                            crate::frame::phase::phase_coherence(&candidate_clone.grid);

                        if coherence >= PHASE_COHERENCE_THRESHOLD {
                            // perform conservative cell-wise phase-aware merge:
                            // iterate cells by index and apply phase merge into base
                            let len = base_slice
                                .grid
                                .cells
                                .len()
                                .min(candidate_clone.grid.cells.len());
                            for i in 0..len {
                                let base_cell = &mut base_slice.grid.cells[i];
                                let cand_cell = &candidate_clone.grid.cells[i];
                                base_cell.apply_phase_merge(cand_cell);

                                // Ensure tags from candidate are propagated into base cell and deduped.
                                if !cand_cell.tags.is_empty() {
                                    base_cell.tags.extend(cand_cell.tags.iter().cloned());
                                    base_cell.tags.sort();
                                    base_cell.tags.dedup();
                                }
                            }

                            // remove candidate slice (sid) from map and insertion order
                            self.slices.remove(&sid);
                            if let Some(pos) =
                                self.slice_order.iter().position(|x| x == &sid)
                            {
                                self.slice_order.remove(pos);
                            }
                        }
                    }
                }
            }
        }

        // after compaction, recompute frame metrics and adaptive rules
        self.auto_adapt();
    }

    // -------------------------------------------------------------------------
    // 🔥 PERCEPTION TRANSFORM (VIEW-DEPENDENT MEMORY)
    // -------------------------------------------------------------------------

    /// Set the current perception mode and recompute the transform.
    pub fn set_perception_mode(&mut self, mode: PerceptionMode) {
        self.perception_mode = mode;
        self.recompute_perception_transform();
    }

    /// Recompute the perception transform based on the current mode and frame metrics.
    fn recompute_perception_transform(&mut self) {
        let mut transform = PerceptionTransform::new(self.perception_mode);

        if self.slices.is_empty() {
            self.perception_transform = transform;
            return;
        }

        for (sid, slice) in &self.slices {
            let mut weight: f32;

            match self.perception_mode {
                PerceptionMode::Default => {
                    // Structural: use average confidence as baseline.
                    weight = slice.average_confidence().max(0.1);
                }
                PerceptionMode::SemanticFocus => {
                    // Emphasize semantic surface and tag richness.
                    let sem = slice.diagonal_semantic_surface();
                    let tag_count: f32 = slice
                        .grid
                        .cells
                        .iter()
                        .map(|c| c.tags.len() as f32)
                        .sum();
                    weight = (sem + tag_count.sqrt()).max(0.1);
                }
                PerceptionMode::TemporalFocus => {
                    // Emphasize recency and temporal alignment.
                    let align = self.temporal_alignment_score;
                    let decay = self.temporal_decay_factor;
                    weight = (align * decay).max(0.1);
                }
                PerceptionMode::DiagonalFocus => {
                    // Emphasize diagonal signature and confidence surface.
                    let diag = slice.diagonal_signature();
                    let conf = slice.diagonal_confidence_surface();
                    weight = (diag + conf).max(0.1);
                }
                PerceptionMode::CrossConnectFocus => {
                    // Emphasize connectivity: number of links touching this slice.
                    let connectivity = self
                        .cross_connect
                        .links_for_slice(sid)
                        .map(|links| links.len() as f32)
                        .unwrap_or(0.0);
                    weight = (connectivity + 1.0).max(0.1);
                }
            }

            transform.slice_weights.insert(sid.clone(), weight);
        }

        self.perception_transform = transform;
    }

    /// Return slice IDs ordered according to the current perception transform.
    /// Higher-weight slices appear earlier; ties fall back to insertion order.
    pub fn perceived_slice_order(&self) -> Vec<SliceId> {
        if self.slices.is_empty() {
            return Vec::new();
        }

        // Build (id, weight, insertion_index) triples to keep deterministic behavior.
        let mut entries: Vec<(SliceId, f32, usize)> = Vec::new();

        for (idx, sid) in self.slice_order.iter().enumerate() {
            let w = self.perception_transform.weight_for(sid);
            entries.push((sid.clone(), w, idx));
        }

        // Sort by weight descending, then by original insertion index ascending.
        entries.sort_by(|a, b| {
            b.1
                .partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        entries.into_iter().map(|(sid, _, _)| sid).collect()
    }
}
















