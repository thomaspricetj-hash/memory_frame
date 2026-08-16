// src/frame/phase.rs

use crate::frame::Grid;

pub fn phase_coherence(grid: &Grid) -> f32 {
    if grid.cells.is_empty() {
        return 0.0;
    }
    let sum_abs: f32 = grid.cells.iter().map(|c| c.phase_net().abs()).sum();
    let avg = sum_abs / (grid.cells.len() as f32).max(1.0);
    avg
}

pub fn phase_entropy_cell(phase_pos: f32, phase_neg: f32) -> f32 {
    let total = (phase_pos + phase_neg).abs().max(1e-6);
    let p = (phase_pos / total).clamp(1e-6, 1.0);
    let q = (phase_neg / total).clamp(1e-6, 1.0);
    - (p * p.ln() + q * q.ln())
}

pub fn phase_entropy(grid: &Grid) -> f32 {
    if grid.cells.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    for c in &grid.cells {
        sum += phase_entropy_cell(c.phase_pos, c.phase_neg);
    }
    sum / (grid.cells.len() as f32).max(1.0)
}






