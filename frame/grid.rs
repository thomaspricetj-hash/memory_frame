// src/frame/grid.rs
//
// Production-ready Grid with serde support and no logic removed.
// Ensure `Cell` and `CellId` types also derive Serialize/Deserialize/Debug/Clone/PartialEq.

use crate::frame::{Cell, CellId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher, DefaultHasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                cells.push(Cell::new(CellId { x, y }));
            }
        }

        Self { width, height, cells }
    }

    /// Return a CellId for coordinates if in bounds.
    pub fn cell_id(&self, x: usize, y: usize) -> Option<CellId> {
        let id = CellId { x, y };
        if self.in_bounds(id) { Some(id) } else { None }
    }

    #[inline]
    pub fn in_bounds(&self, id: CellId) -> bool {
        id.x < self.width && id.y < self.height
    }

    #[inline]
    pub fn index(&self, id: CellId) -> usize {
        debug_assert!(self.in_bounds(id));
        id.y * self.width + id.x
    }

    #[inline]
    pub fn get_cell(&self, id: CellId) -> Option<&Cell> {
        if !self.in_bounds(id) {
            return None;
        }
        self.cells.get(self.index(id))
    }

    #[inline]
    pub fn get_cell_mut(&mut self, id: CellId) -> Option<&mut Cell> {
        if !self.in_bounds(id) {
            return None;
        }
        let idx = self.index(id);
        self.cells.get_mut(idx)
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn average_confidence(&self) -> f32 {
        if self.cells.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.cells.iter().map(|c| c.confidence).sum();
        sum / self.cells.len() as f32
    }

    // ---------------------------------------------------------
    // ðŸ”¥ DIAGONAL LAW OF THE NINE-MATRIX (MAX TIER)
    // ---------------------------------------------------------

    /// Returns the 4 orthogonal neighbors (Von Neumann).
    pub fn neighbors(&self, id: CellId) -> Vec<CellId> {
        let mut out = Vec::with_capacity(4);

        let dirs = [
            (1_i32, 0_i32),
            (-1_i32, 0_i32),
            (0_i32, 1_i32),
            (0_i32, -1_i32),
        ];

        for (dx, dy) in dirs {
            let nx = id.x as i32 + dx;
            let ny = id.y as i32 + dy;

            if nx >= 0 && ny >= 0 {
                let cid = CellId { x: nx as usize, y: ny as usize };
                if self.in_bounds(cid) {
                    out.push(cid);
                }
            }
        }

        out
    }

    /// ðŸ”¥ Returns the 4 diagonal neighbors (Nine-Matrix diagonals).
    pub fn diagonal_neighbors(&self, id: CellId) -> Vec<CellId> {
        let mut out = Vec::with_capacity(4);

        let dirs = [
            (1_i32, 1_i32),
            (1_i32, -1_i32),
            (-1_i32, 1_i32),
            (-1_i32, -1_i32),
        ];

        for (dx, dy) in dirs {
            let nx = id.x as i32 + dx;
            let ny = id.y as i32 + dy;

            if nx >= 0 && ny >= 0 {
                let cid = CellId { x: nx as usize, y: ny as usize };
                if self.in_bounds(cid) {
                    out.push(cid);
                }
            }
        }

        out
    }

    /// ðŸ”¥ Diagonal weight: how strong diagonal influence is for a cell.
    pub fn diagonal_weight(&self, id: CellId) -> f32 {
        let cx = (self.width as f32) / 2.0;
        let cy = (self.height as f32) / 2.0;

        let dx = (id.x as f32 - cx).abs();
        let dy = (id.y as f32 - cy).abs();

        let diag_dist = (dx - dy).abs();

        let w = 1.0 / (1.0 + diag_dist);
        w.clamp(0.25, 1.0)
    }

    /// ðŸ”¥ Diagonal semantic boost: increases tag influence along diagonals.
    pub fn diagonal_tag_boost(&self, id: CellId, tag: &str) -> f32 {
        let w = self.diagonal_weight(id);

        let base = if tag.len() > 6 { 1.2 } else { 1.0 };

        (base * w).clamp(1.0, 2.0)
    }

    /// ðŸ”¥ Diagonal confidence propagation: blends confidence across diagonals.
    pub fn diagonal_confidence(&self, id: CellId) -> f32 {
        let diag = self.diagonal_neighbors(id);

        if diag.is_empty() {
            return self.get_cell(id).map(|c| c.confidence).unwrap_or(0.0);
        }

        let mut sum = 0.0;
        let mut count = 0;

        for cid in diag {
            if let Some(c) = self.get_cell(cid) {
                sum += c.confidence;
                count += 1;
            }
        }

        let avg = if count == 0 { 0.0 } else { sum / count as f32 };

        let own = self.get_cell(id).map(|c| c.confidence).unwrap_or(0.0);

        (own * 0.6) + (avg * 0.4)
    }

    /// ðŸ”¥ BitDropâ€‘V2 diagonal block signature (max-tier hybrid)
    pub fn diagonal_block_signature(&self, id: CellId) -> u64 {
        let mut hasher = DefaultHasher::new();

        if let Some(c) = self.get_cell(id) {
            c.confidence.to_bits().hash(&mut hasher);
            for t in &c.tags {
                t.hash(&mut hasher);
            }
        }

        for cid in self.diagonal_neighbors(id) {
            if let Some(c) = self.get_cell(cid) {
                c.confidence.to_bits().hash(&mut hasher);
                for t in &c.tags {
                    t.hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }

    /// ðŸ”¥ Diagonal influence map: returns diagonal weight for every cell.
    pub fn diagonal_map(&self) -> Vec<(CellId, f32)> {
        self.cells
            .iter()
            .map(|c| (c.id, self.diagonal_weight(c.id)))
            .collect()
    }

    // ---------------------------------------------------------
    // Existing utilities (kept intact)
    // ---------------------------------------------------------

    pub fn dominant_tags(&self) -> Vec<String> {
        let mut freq: HashMap<String, usize> = HashMap::new();

        for cell in &self.cells {
            for tag in &cell.tags {
                *freq.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let mut items: Vec<(String, usize)> = freq.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));

        items.into_iter().map(|(tag, _)| tag).take(5).collect()
    }

    pub fn high_confidence_cells(&self, threshold: f32) -> Vec<CellId> {
        self.cells
            .iter()
            .filter(|c| c.confidence >= threshold)
            .map(|c| c.id)
            .collect()
    }

    pub fn tag_histogram(&self) -> Vec<(String, usize)> {
        let mut freq: HashMap<String, usize> = HashMap::new();

        for cell in &self.cells {
            for tag in &cell.tags {
                *freq.entry(tag.clone()).or_insert(0) += 1;
            }
        }

        let mut items: Vec<(String, usize)> = freq.into_iter().collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items
    }

    pub fn strongest_cell(&self) -> Option<&Cell> {
        self.cells.iter().max_by(|a, b| a.confidence.total_cmp(&b.confidence))
    }

    pub fn weakest_cell(&self) -> Option<&Cell> {
        self.cells.iter().min_by(|a, b| a.confidence.total_cmp(&b.confidence))
    }

    // ---------------------------------------------------------------------
    // Helpers required by harmonics module
    // ---------------------------------------------------------------------

    pub fn clone_cells_phase_net_vec(&self) -> Vec<f32> {
        self.cells.iter().map(|c| c.phase_net()).collect()
    }

    pub fn collect_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        for c in &self.cells {
            for t in &c.tags {
                if !tags.contains(t) {
                    tags.push(t.clone());
                }
            }
        }
        tags
    }
}







