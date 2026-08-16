#![allow(dead_code)]

//! MemoryFrame Engine
//! Multi-layer cognitive memory system with grid cells,
//! cross-connects, evolution, conflict resolution, compression,
//! storage, visualization, and API adapters.

pub mod config;
pub mod frame;
pub mod layers;
pub mod storage;
pub mod viz;
pub mod api;

// Config
pub use config::MemoryPolicy;

// Core frame types and compression helpers
pub use frame::{
    MemoryFrame,
    Slice,
    SliceId,
    SliceData,
    Grid,
    Cell,
    CellId,
    CrossConnect,
    // BitDrop compression helpers (re-exported from frame module)
    compress_slice_max,
    decompress_slice_max,
    save_compressed_max,
    load_compressed_max,
    compress_and_report,
};

// Layers
pub use layers::{
    LayerId,
    VisualLayer,
    SemanticLayer,
    TemporalLayer,
    EmotionalLayer,
    RelationalLayer,
    DeclarativeLayer,
};

// Storage (only what actually exists)
pub use storage::*;

// Visualization (only what actually exists)
pub use viz::*;

// API
pub use api::{
    ModelAdapter,
    FrameQuery,
    SliceQuery,
    CellQuery,
    FrameSummary,
    SliceSummary,
    ApiError,
};

/// Create a new memory frame with default policy.
pub fn new_frame() -> MemoryFrame {
    MemoryFrame::new(config::defaults::default_policy())
}

/// Create a new memory frame with a custom policy.
pub fn new_frame_with_policy(policy: MemoryPolicy) -> MemoryFrame {
    MemoryFrame::new(policy)
}







