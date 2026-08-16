use std::collections::HashMap;
use crate::frame::CellId;

/// A directional weighted link between two cells.
#[derive(Debug, Clone)]
pub struct Link {
    pub from: CellId,
    pub to: CellId,
    pub weight: f32,

    /// Diagonal propagation weight (Nine‑Matrix style) for this link.
    pub diagonal_weight: f32,

    /// Collapse influence: how strongly this link participates in
    /// multi‑hop collapse / convergence.
    pub collapse_influence: f32,
}

/// Cross-layer relational graph.
/// Each cell may have multiple outgoing links.
#[derive(Debug, Clone)]
pub struct CrossConnect {
    /// Mapping: CellId → outgoing links
    pub links: HashMap<CellId, Vec<Link>>,

    /// Diagonal frame signature: a hash‑like scalar summarizing
    /// the diagonal structure of the cross‑layer graph.
    pub diagonal_frame_signature: f32,

    /// Diagonal semantic surface: aggregate diagonal weight across all links.
    pub diagonal_semantic_surface: f32,

    /// Diagonal temporal alignment: synthetic scalar capturing how
    /// well the graph aligns along diagonal propagation paths.
    pub diagonal_temporal_alignment: f32,

    /// Diagonal propagation weight: global scaling factor for
    /// diagonal‑aware traversal and reasoning.
    pub diagonal_propagation_weight: f32,

    /// Diagonal collapse influence: global scalar used to modulate
    /// multi‑hop collapse / convergence behavior.
    pub diagonal_collapse_influence: f32,
}

impl CrossConnect {
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
            diagonal_frame_signature: 0.0,
            diagonal_semantic_surface: 0.0,
            diagonal_temporal_alignment: 0.0,
            diagonal_propagation_weight: 1.0,
            diagonal_collapse_influence: 1.0,
        }
    }

    /// Add a link, avoiding duplicates and normalizing weight.
    /// Also computes per‑link diagonal weight and collapse influence.
    pub fn add_link(&mut self, from: CellId, to: CellId, weight: f32) {
        let weight = weight.clamp(0.0, 1.0);

        let diag_w = diagonal_weight_for_cells(from, to);
        let collapse_inf = collapse_influence_for_link(weight, diag_w);

        let entry = self.links.entry(from).or_default();

        // Avoid duplicate links
        if let Some(existing) = entry.iter_mut().find(|l| l.to == to) {
            existing.weight = existing.weight.max(weight);
            existing.diagonal_weight = existing.diagonal_weight.max(diag_w);
            existing.collapse_influence = existing.collapse_influence.max(collapse_inf);
            self.recompute_diagonal_metrics();
            return;
        }

        entry.push(Link {
            from,
            to,
            weight,
            diagonal_weight: diag_w,
            collapse_influence: collapse_inf,
        });

        self.recompute_diagonal_metrics();
    }

    /// Remove a link if it exists.
    pub fn remove_link(&mut self, from: CellId, to: CellId) {
        if let Some(vec) = self.links.get_mut(&from) {
            vec.retain(|l| l.to != to);
        }
        self.recompute_diagonal_metrics();
    }

    /// Get all outgoing link targets from a cell.
    pub fn links_from(&self, id: CellId) -> Vec<CellId> {
        self.links
            .get(&id)
            .map(|v| v.iter().map(|l| l.to).collect())
            .unwrap_or_default()
    }

    /// Get weighted outgoing links.
    pub fn weighted_links_from(&self, id: CellId) -> Vec<(CellId, f32)> {
        self.links
            .get(&id)
            .map(|v| v.iter().map(|l| (l.to, l.weight)).collect())
            .unwrap_or_default()
    }

    /// Returns all links that touch the frame; slice_id is accepted
    /// for API compatibility but not used for filtering (no LayerId
    /// information is present in CellId).
    pub fn links_for_slice(
        &self,
        _slice_id: &crate::layers::LayerId,
    ) -> Option<Vec<Link>> {
        let mut out = Vec::new();

        for vec in self.links.values() {
            out.extend(vec.iter().cloned());
        }

        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Multi-hop traversal with hop-aware decay and diagonal propagation.
    pub fn traverse_multi_hop(
        &self,
        start: CellId,
        max_hops: usize,
        decay_per_hop: f32,
    ) -> Vec<(CellId, f32)> {
        let mut results = Vec::new();
        let mut frontier = vec![(start, 1.0)];

        for hop in 0..max_hops {
            let mut next_frontier = Vec::new();

            for (node, weight) in frontier {
                if let Some(outgoing) = self.links.get(&node) {
                    for link in outgoing {
                        let hop_decay = 1.0 - (decay_per_hop * hop as f32);

                        let diag_factor =
                            link.diagonal_weight * self.diagonal_propagation_weight;
                        let collapse_factor =
                            link.collapse_influence * self.diagonal_collapse_influence;

                        let new_weight =
                            weight * link.weight * hop_decay * diag_factor * collapse_factor;

                        results.push((link.to, new_weight));
                        next_frontier.push((link.to, new_weight));
                    }
                }
            }

            frontier = next_frontier;

            if frontier.is_empty() {
                break;
            }
        }

        results
    }

    /// Normalize all link weights so each cell's outgoing links sum to 1.0.
    pub fn normalize(&mut self) {
        for (_id, vec) in self.links.iter_mut() {
            let sum: f32 = vec.iter().map(|l| l.weight).sum();
            if sum > 0.0 {
                for link in vec.iter_mut() {
                    link.weight /= sum;
                }
            }
        }
        self.recompute_diagonal_metrics();
    }

    /// Recompute diagonal frame signature, semantic surface,
    /// temporal alignment, propagation weight, and collapse influence.
    fn recompute_diagonal_metrics(&mut self) {
        let mut total_diag = 0.0;
        let mut total_weight = 0.0;
        let mut total_collapse = 0.0;
        let mut count = 0.0;

        for (from, vec) in &self.links {
            for link in vec {
                total_diag += link.diagonal_weight;

                let dx = (link.to.x as f32 - from.x as f32).abs();
                let dy = (link.to.y as f32 - from.y as f32).abs();
                total_weight += (dx + dy) * link.weight;

                total_collapse += link.collapse_influence;
                count += 1.0;
            }
        }

        if count == 0.0 {
            self.diagonal_frame_signature = 0.0;
            self.diagonal_semantic_surface = 0.0;
            self.diagonal_temporal_alignment = 0.0;
            self.diagonal_propagation_weight = 1.0;
            self.diagonal_collapse_influence = 1.0;
            return;
        }

        self.diagonal_semantic_surface = total_diag / count;
        self.diagonal_frame_signature = total_weight / count;

        self.diagonal_temporal_alignment =
            1.0 - (self.diagonal_semantic_surface - 0.5).abs().clamp(0.0, 1.0);

        self.diagonal_propagation_weight =
            (self.diagonal_semantic_surface + self.diagonal_temporal_alignment) / 2.0;

        self.diagonal_collapse_influence = (total_collapse / count).clamp(0.25, 2.0);
    }
}

/// Per-link diagonal weight based on cell coordinates.
fn diagonal_weight_for_cells(from: CellId, to: CellId) -> f32 {
    let dx = (to.x as f32 - from.x as f32).abs();
    let dy = (to.y as f32 - from.y as f32).abs();

    if dx == 0.0 && dy == 0.0 {
        return 0.0;
    }

    let diag_dist = (dx - dy).abs();
    let w = 1.0 / (1.0 + diag_dist);
    w.clamp(0.25, 1.5)
}

/// Collapse influence for a link based on its weight and diagonal strength.
fn collapse_influence_for_link(weight: f32, diag_w: f32) -> f32 {
    let base = weight;
    let diag = diag_w;
    (base * diag).clamp(0.1, 2.0)
}









