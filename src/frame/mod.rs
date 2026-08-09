pub mod cell;
pub mod grid;
pub mod slice;
pub mod cross_connect;
pub mod memory_frame;
pub mod compression;
pub mod navigation;
pub mod adaptive_rules;
pub mod harmonics;
pub mod phase;

pub use cell::{Cell, CellId};
pub use grid::Grid;
pub use slice::{Slice, SliceId, SliceData};
pub use cross_connect::{CrossConnect, Link};
pub use memory_frame::MemoryFrame;

pub use compression::{
    compress_slice_max,
    decompress_slice_max,
    save_compressed_max,
    load_compressed_max,
    compress_and_report,
};
