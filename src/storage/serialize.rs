use anyhow::{Result, anyhow};
use serde_json;
use chrono::Utc;
use crc32fast::Hasher;

use crate::frame::MemoryFrame;
use crate::storage::schema::{FrameRecord, SliceRecord, CellRecord};

pub fn serialize_frame(frame: &MemoryFrame) -> Result<FrameRecord> {
    let mut slices = Vec::new();

    for (id, slice) in &frame.slices {
        let mut cells = Vec::new();

        for cell in &slice.grid.cells {
            // Compute cell checksum
            let mut hasher = Hasher::new();
            hasher.update(&cell.confidence.to_le_bytes());
            for tag in &cell.tags {
                hasher.update(tag.as_bytes());
            }
            let checksum = hasher.finalize();

            cells.push(CellRecord {
                id: cell.id,
                confidence: cell.confidence,
                tags: cell.tags.clone(),
                timestamp: Utc::now(),
                checksum,
                version: 1,
            });
        }

        let data_json = serde_json::to_string(&slice.data)?;

        // Compute slice checksum
        let mut hasher = Hasher::new();
        hasher.update(data_json.as_bytes());
        let slice_checksum = hasher.finalize();

        slices.push(SliceRecord {
            id: id.clone(),
            data: data_json,
            width: slice.grid.width,
            height: slice.grid.height,
            cells,
            timestamp: Utc::now(),
            checksum: slice_checksum,
            cell_count: slice.grid.cells.len(),
            version: 1,
        });
    }

    // Compute frame checksum
    let mut hasher = Hasher::new();
    for slice in &slices {
        hasher.update(slice.checksum.to_le_bytes());
    }
    let frame_checksum = hasher.finalize();

    Ok(FrameRecord {
        id: frame.id,
        slices,
        version: 1,
        timestamp: Utc::now(),
        checksum: frame_checksum,
        slice_count: frame.slices.len(),
    })
}
