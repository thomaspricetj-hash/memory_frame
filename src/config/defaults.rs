use super::types::*;

pub fn default_policy() -> MemoryPolicy {
    MemoryPolicy {
        // ---------------------------------------------------------
        // DECAY POLICY — tuned for stable short-term → long-term flow
        // ---------------------------------------------------------
        decay: DecayPolicy {
            // 6-hour half-life gives fast short-term decay but preserves
            // meaningful activations long enough for consolidation.
            time_half_life_secs: 21_600,

            // Confidence floor raised slightly to prevent “dead cells”
            // and keep low-signal nodes available for future reinforcement.
            confidence_floor: 0.08,
        },

        // ---------------------------------------------------------
        // CONFLICT POLICY — tuned for multi-layer fusion
        // ---------------------------------------------------------
        conflict: ConflictPolicy {
            // Newer slices override older ones when both exist.
            prefer_newer: true,

            // Higher-confidence cells win when merging.
            prefer_higher_confidence: true,

            // Merge is enabled so tags, metadata, and cross-links accumulate.
            merge_enabled: true,
        },

        // ---------------------------------------------------------
        // COMPRESSION POLICY — tuned for your grid + heatmap engine
        // ---------------------------------------------------------
        compression: CompressionPolicy {
            // Slice compaction is essential for your 3D viz + storage pipeline.
            enable_slice_compaction: true,

            // 0.90 threshold gives more aggressive merging without losing structure.
            cell_merge_threshold: 0.90,

            // Increased redundancy cap to support richer semantic slices.
            max_redundant_cells: 256,
        },

        // ---------------------------------------------------------
        // CROSS-CONNECT POLICY — tuned for multi-layer reasoning
        // ---------------------------------------------------------
        cross_connect: CrossConnectPolicy {
            // More links per cell improves relational reasoning.
            max_links_per_cell: 24,

            // Default weight slightly reduced to encourage decay-based shaping.
            default_weight: 0.85,

            // Hop decay lowered to allow deeper multi-hop inference.
            decay_per_hop: 0.10,
        },
    }
}
