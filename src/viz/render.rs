use glam::Vec3;
use crate::viz::SlicePosition;

/// RenderPacket is the GPU‑ready representation of a slice.
/// Max‑tier upgrade:
/// - deterministic color based on ID hash
/// - stable scale curve
/// - zero breakage (no new fields required)
#[derive(Debug, Clone)]
pub struct RenderPacket {
    pub id: String,
    pub position: Vec3,
    pub color: [f32; 3],
    pub scale: f32,
}

impl From<&SlicePosition> for RenderPacket {
    fn from(sp: &SlicePosition) -> Self {
        // Deterministic color derived from slice ID
        let color = color_from_id(&sp.id);

        // Stable scale based on Z-depth (visual layering)
        let scale = 0.8 + ((sp.z + 1.0) / 2.0) * 0.4;

        RenderPacket {
            id: sp.id.clone(),
            position: Vec3::new(sp.x, sp.y, sp.z),
            color,
            scale,
        }
    }
}

/// Deterministic color generation based on ID hash.
/// This gives each slice a unique but stable color.
fn color_from_id(id: &str) -> [f32; 3] {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    let hash = h.finish();

    // Convert hash → RGB in 0.2–1.0 range
    let r = (((hash >> 16) & 0xFF) as f32 / 255.0).clamp(0.2, 1.0);
    let g = (((hash >> 8)  & 0xFF) as f32 / 255.0).clamp(0.2, 1.0);
    let b = (( hash        & 0xFF) as f32 / 255.0).clamp(0.2, 1.0);

    [r, g, b]
}

