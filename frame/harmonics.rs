// src/frame/harmonics.rs

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::frame::Grid;

pub fn pool_2x2(grid: &Grid) -> Vec<f32> {
    let w = grid.width / 2;
    let h = grid.height / 2;
    let mut out = Vec::with_capacity(w.saturating_mul(h));
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0f32;
            let mut cnt = 0usize;
            for oy in 0..2 {
                for ox in 0..2 {
                    let gx = x * 2 + ox;
                    let gy = y * 2 + oy;
                    if gx < grid.width && gy < grid.height {
                        let idx = gy * grid.width + gx;
                        if let Some(c) = grid.cells.get(idx) {
                            sum += c.phase_net();
                            cnt += 1;
                        }
                    }
                }
            }
            out.push(if cnt > 0 { sum / cnt as f32 } else { 0.0 });
        }
    }
    out
}

pub fn multi_scale_pools(grid: &Grid, levels: usize) -> Vec<Vec<f32>> {
    let mut levels_out = Vec::with_capacity(levels);
    let mut current = grid.clone_cells_phase_net_vec();
    let mut cur_w = grid.width;
    let mut cur_h = grid.height;
    levels_out.push(current.clone());
    for _ in 1..levels {
        if cur_w < 2 || cur_h < 2 {
            break;
        }
        let mut pooled = Vec::with_capacity((cur_w / 2) * (cur_h / 2));
        for y in 0..(cur_h / 2) {
            for x in 0..(cur_w / 2) {
                let mut sum = 0.0f32;
                let mut cnt = 0usize;
                for oy in 0..2 {
                    for ox in 0..2 {
                        let sx = x * 2 + ox;
                        let sy = y * 2 + oy;
                        let idx = sy * cur_w + sx;
                        if let Some(v) = current.get(idx) {
                            sum += *v;
                            cnt += 1;
                        }
                    }
                }
                pooled.push(if cnt > 0 { sum / cnt as f32 } else { 0.0 });
            }
        }
        levels_out.push(pooled.clone());
        current = pooled;
        cur_w = cur_w / 2;
        cur_h = cur_h / 2;
    }
    levels_out
}

pub fn compute_low_freq_avg(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0f32;
    let take = (values.len() / 4).max(1);
    for i in 0..take {
        sum += values[i];
    }
    sum / take as f32
}

pub fn harmonic_signature(grid: &Grid, tags: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    let base_vec = grid.clone_cells_phase_net_vec();
    let avg0 = if base_vec.is_empty() { 0.0 } else { base_vec.iter().copied().sum::<f32>() / base_vec.len() as f32 };
    (avg0.to_bits()).hash(&mut hasher);
    let pools = multi_scale_pools(grid, 3);
    for p in pools.iter().take(3) {
        let avg = compute_low_freq_avg(p);
        (avg.to_bits()).hash(&mut hasher);
    }
    for t in tags {
        t.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn octave_index_from_distance(dist: f32) -> i32 {
    if dist <= 0.0 {
        return 0;
    }
    let mut idx = 0i32;
    let mut d = dist;
    while d > 1.0 {
        d /= 2.0;
        idx += 1;
    }
    idx
}

pub fn radial_harmonic_weight(center_x: f32, center_y: f32, x: f32, y: f32) -> f32 {
    let dx = x - center_x;
    let dy = y - center_y;
    let dist = (dx * dx + dy * dy).sqrt();
    let octave = octave_index_from_distance(dist) as f32;
    1.0 / (1.0 + octave)
}






