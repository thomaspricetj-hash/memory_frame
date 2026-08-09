// src/frame/slice.rs

use crate::frame::Grid;
use crate::layers::LayerId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type SliceId = LayerId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SliceData {
    Visual(Vec<u8>),
    Semantic(serde_json::Value),
    Temporal(DateTime<Utc>),
    Emotional(f32),
    Relational(Vec<f32>),
    Declarative(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slice {
    pub id: SliceId,
    pub grid: Grid,
    pub data: SliceData,
}

impl Slice {
    /// Create a new slice with a default 32×32 grid.
    pub fn new(id: SliceId, data: SliceData) -> Self {
        Self {
            id,
            grid: Grid::new(32, 32),
            data,
        }
    }

    /// Create a slice with a custom grid size.
    pub fn with_size(id: SliceId, data: SliceData, width: usize, height: usize) -> Self {
        Self {
            id,
            grid: Grid::new(width, height),
            data,
        }
    }

    pub fn example_with_id(id: SliceId) -> Self {
        let mut grid = Grid::new(8, 8);
        if let Some(cell_id) = grid.cell_id(1, 1) {
            if let Some(cell) = grid.get_cell_mut(cell_id) {
                cell.confidence = 0.9;
                cell.tags.push("example".to_string());
            }
        }
        if let Some(cell_id) = grid.cell_id(6, 6) {
            if let Some(cell) = grid.get_cell_mut(cell_id) {
                cell.confidence = 0.2;
                cell.tags.push("low".to_string());
            }
        }

        Self {
            id,
            grid,
            data: SliceData::Declarative("example slice".to_string()),
        }
    }

    pub fn example() -> Self {
        if let Some(id) = LayerId::from_str_fast("example") {
            Self::example_with_id(id)
        } else {
            panic!(
                "Slice::example() could not construct a LayerId. \
                 Implement Default for LayerId, provide a constructor, or use \
                 Slice::example_with_id(id) in tests."
            )
        }
    }

    /// Average confidence across all cells in this slice.
    pub fn average_confidence(&self) -> f32 {
        self.grid.average_confidence()
    }

    /// Extract the top tags from this slice.
    pub fn dominant_tags(&self) -> Vec<String> {
        self.grid.dominant_tags()
    }

    /// Count total cells.
    pub fn cell_count(&self) -> usize {
        self.grid.cell_count()
    }

    /// Return all high‑confidence cells above a threshold.
    pub fn high_confidence_cells(&self, threshold: f32) -> Vec<crate::frame::CellId> {
        self.grid.high_confidence_cells(threshold)
    }

    /// Return the strongest cell in the slice.
    pub fn strongest_cell(&self) -> Option<&crate::frame::Cell> {
        self.grid.strongest_cell()
    }

    /// Return the weakest cell in the slice.
    pub fn weakest_cell(&self) -> Option<&crate::frame::Cell> {
        self.grid.weakest_cell()
    }

    /// Return all tags sorted by frequency.
    pub fn tag_histogram(&self) -> Vec<(String, usize)> {
        self.grid.tag_histogram()
    }

    /// Update slice metadata (Semantic, Declarative, etc.)
    pub fn update_data(&mut self, new_data: SliceData) {
        self.data = new_data;
    }

    /// Diagonal slice signature using strongest + weakest cells only.
    pub fn diagonal_signature(&self) -> f32 {
        let w = self.grid_width() as f32;
        let h = self.grid_height() as f32;

        let cx = (w - 1.0) / 2.0;
        let cy = (h - 1.0) / 2.0;

        let mut score = 0.0;
        let mut count = 0.0;

        if let Some(cell) = self.strongest_cell() {
            let dx = (cell.id.x as f32 - cx).abs();
            let dy = (cell.id.y as f32 - cy).abs();
            let diag = 1.0 / (1.0 + (dx - dy).abs());
            score += diag * cell.confidence;
            count += 1.0;
        }

        if let Some(cell) = self.weakest_cell() {
            let dx = (cell.id.x as f32 - cx).abs();
            let dy = (cell.id.y as f32 - cy).abs();
            let diag = 1.0 / (1.0 + (dx - dy).abs());
            score += diag * cell.confidence;
            count += 1.0;
        }

        if count == 0.0 { 0.0 } else { score / count }
    }

    /// Diagonal semantic surface using tag histogram density.
    pub fn diagonal_semantic_surface(&self) -> f32 {
        let tags = self.tag_histogram();
        if tags.is_empty() {
            return 0.0;
        }

        let total_tags: usize = tags.iter().map(|(_, c)| *c).sum();
        let density = total_tags as f32 / self.cell_count().max(1) as f32;

        density.clamp(0.0, 1.5)
    }

    /// Diagonal confidence surface using average confidence and grid aspect ratio.
    pub fn diagonal_confidence_surface(&self) -> f32 {
        let avg = self.average_confidence();
        let w = self.grid_width() as f32;
        let h = self.grid_height() as f32;

        let aspect_diag = 1.0 / (1.0 + (w - h).abs());
        (avg * aspect_diag).clamp(0.0, 1.5)
    }

    /// Diagonal propagation weight: semantic + confidence blend.
    pub fn diagonal_propagation_weight(&self) -> f32 {
        let sem = self.diagonal_semantic_surface();
        let conf = self.diagonal_confidence_surface();
        ((sem + conf) / 2.0).clamp(0.25, 1.5)
    }

    /// Diagonal collapse influence: signature × confidence.
    pub fn diagonal_collapse_influence(&self) -> f32 {
        let sig = self.diagonal_signature();
        (sig * self.average_confidence()).clamp(0.1, 2.0)
    }

    /// Helper: grid width (field access, not method).
    fn grid_width(&self) -> usize {
        self.grid.width
    }

    /// Helper: grid height (field access, not method).
    fn grid_height(&self) -> usize {
        self.grid.height
    }

    /// Compute a compact harmonic signature for this slice using multi-scale pools and tags.
    pub fn harmonic_signature(&self) -> u64 {
        crate::frame::harmonics::harmonic_signature(&self.grid, &self.grid.collect_tags())
    }

    /// Initialize phase channels from legacy confidence values for all cells.
    pub fn init_phases_from_confidence(&mut self) {
        for c in &mut self.grid.cells {
            c.init_phases_from_confidence();
        }
    }
}


