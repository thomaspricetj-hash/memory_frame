use serde::{Serialize, Deserialize};

/// Controls temporal decay of cell confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayPolicy {
    /// Half-life in seconds for exponential decay.
    /// Lower values = faster forgetting, higher = longer retention.
    pub time_half_life_secs: u64,

    /// Minimum confidence a cell can decay to.
    /// Prevents â€œdead cellsâ€ and supports long-term stabilization.
    pub confidence_floor: f32,
}

/// Controls how conflicting cells are resolved during merges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictPolicy {
    /// Prefer newer slices when merging.
    pub prefer_newer: bool,

    /// Prefer higher-confidence cells when merging.
    pub prefer_higher_confidence: bool,

    /// Whether merging is enabled at all.
    /// If false, conflicts result in replacement instead of fusion.
    pub merge_enabled: bool,
}

/// Controls compression, compaction, and redundancy reduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPolicy {
    /// Whether slice compaction is enabled.
    pub enable_slice_compaction: bool,

    /// Threshold for merging similar cells.
    /// 1.0 = identical only, 0.0 = merge everything.
    pub cell_merge_threshold: f32,

    /// Maximum number of redundant cells allowed before compaction triggers.
    pub max_redundant_cells: usize,
}

/// Controls cross-layer relational links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossConnectPolicy {
    /// Maximum number of links a cell may hold.
    pub max_links_per_cell: usize,

    /// Default weight assigned to new links.
    pub default_weight: f32,

    /// How much link weight decays per hop during multi-hop reasoning.
    pub decay_per_hop: f32,
}

/// Unified policy object controlling all memory behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub decay: DecayPolicy,
    pub conflict: ConflictPolicy,
    pub compression: CompressionPolicy,
    pub cross_connect: CrossConnectPolicy,
}






