use glam::{Vec3};
use crate::frame::{Slice, Cell};
use crate::viz::{LayerColor, ColorMap};

#[derive(Debug)]
pub struct RenderPacket {
    pub slice_id: String,
    pub cell_positions: Vec<Vec3>,
    pub cell_colors: Vec<LayerColor>,
}

impl RenderPacket {
    pub fn from_slice(slice: &Slice) -> Self {
        let mut positions = Vec::new();
        let mut colors = Vec::new();

        let color = ColorMap::for_layer(&slice.id);

        for cell in &slice.grid.cells {
            let x = cell.id.x as f32;
            let y = cell.id.y as f32;
            let pos = Vec3::new(x, y, 0.0);

            positions.push(pos);

            let mut c = color.clone();
            let alpha = ColorMap::confidence_to_alpha(cell.confidence);
            c.r *= alpha;
            c.g *= alpha;
            c.b *= alpha;

            colors.push(c);
        }

        Self {
            slice_id: slice.id.to_string(),
            cell_positions: positions,
            cell_colors: colors,
        }
    }
}






