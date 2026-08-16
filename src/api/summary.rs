use crate::frame::{MemoryFrame, Slice};

/// Highâ€‘level summary of an entire memory frame.
pub struct FrameSummary {
    pub description: String,
    pub layers_present: Vec<String>,
    pub confidence: f32,
    pub slice_count: usize,
    pub cell_count: usize,
}

/// Summary of a single slice.
pub struct SliceSummary {
    pub description: String,
    pub cell_count: usize,
    pub dominant_tags: Vec<String>,
    pub avg_confidence: f32,
}

impl FrameSummary {
    pub fn from_frame(frame: &MemoryFrame) -> Self {
        let layers_present = frame
            .slices
            .keys()
            .map(|id| id.to_string())
            .collect::<Vec<_>>();

        let slice_count = layers_present.len();

        let cell_count = frame
            .slices
            .values()
            .map(|slice| slice.grid.cells.len())
            .sum();

        Self {
            description: format!("Frame with {} layers", slice_count),
            layers_present,
            confidence: frame.global_confidence(),
            slice_count,
            cell_count,
        }
    }
}

impl SliceSummary {
    pub fn from_slice(slice: &Slice) -> Self {
        let layer_name = slice.id.to_string();

        let cell_count = slice.grid.cell_count();
        let avg_confidence = slice.grid.average_confidence();
        let dominant_tags = slice.grid.dominant_tags();

        Self {
            description: format!("Slice type: {}", layer_name),
            cell_count,
            dominant_tags,
            avg_confidence,
        }
    }
}






