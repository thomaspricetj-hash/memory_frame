use crate::frame::{MemoryFrame, CellId};
use crate::config::MemoryPolicy;
use chrono::Utc;

impl MemoryFrame {
    /// Resolve conflict between two cells across all slices.
    /// Winner chosen using policy rules + confidence + recency.
    pub fn resolve_conflict(
        &mut self,
        cell_a: CellId,
        cell_b: CellId,
    ) -> Option<CellId> {
        let policy: &MemoryPolicy = &self.policy;

        // Aggregate confidence across all slices
        let (conf_a, conf_b) = self.aggregate_confidence(cell_a, cell_b);

        // Aggregate recency (newer cell wins if enabled)
        let (ts_a, ts_b) = self.aggregate_recency(cell_a, cell_b);

        let mut winner = cell_a;
        let mut loser = cell_b;

        // ---------------------------------------------------------
        // 1. Prefer newer cell (if enabled)
        // ---------------------------------------------------------
        if policy.conflict.prefer_newer {
            match (ts_a, ts_b) {
                (Some(a), Some(b)) if b > a => {
                    winner = cell_b;
                    loser = cell_a;
                }
                (None, Some(_)) => {
                    winner = cell_b;
                    loser = cell_a;
                }
                _ => {}
            }
        }

        // ---------------------------------------------------------
        // 2. Prefer higher confidence (if enabled)
        // ---------------------------------------------------------
        if policy.conflict.prefer_higher_confidence && conf_b > conf_a {
            winner = cell_b;
            loser = cell_a;
        }

        // ---------------------------------------------------------
        // 3. Merge loser into winner (if enabled)
        // ---------------------------------------------------------
        if policy.conflict.merge_enabled {
            self.merge_cells(winner, loser);
        }

        Some(winner)
    }

    /// Aggregate confidence across all slices.
    fn aggregate_confidence(&self, a: CellId, b: CellId) -> (f32, f32) {
        let mut conf_a = 0.0;
        let mut conf_b = 0.0;

        for slice in self.slices.values() {
            if let Some(ca) = slice.grid.get_cell(a) {
                conf_a += ca.confidence;
            }
            if let Some(cb) = slice.grid.get_cell(b) {
                conf_b += cb.confidence;
            }
        }

        (conf_a, conf_b)
    }

    /// Aggregate recency across slices (newest timestamp wins).
    fn aggregate_recency(&self, a: CellId, b: CellId) -> (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>) {
        let mut ts_a: Option<chrono::DateTime<Utc>> = None;
        let mut ts_b: Option<chrono::DateTime<Utc>> = None;

        for slice in self.slices.values() {
            if let Some(ca) = slice.grid.get_cell(a) {
                if let Some(ts) = ca.last_updated {
                    ts_a = match ts_a {
                        Some(prev) if prev > ts => Some(prev),
                        _ => Some(ts),
                    };
                }
            }

            if let Some(cb) = slice.grid.get_cell(b) {
                if let Some(ts) = cb.last_updated {
                    ts_b = match ts_b {
                        Some(prev) if prev > ts => Some(prev),
                        _ => Some(ts),
                    };
                }
            }
        }

        (ts_a, ts_b)
    }

    /// Merge loser cell into winner cell across all slices.
    /// Borrowâ€‘safe, metadataâ€‘aware, timestampâ€‘aware.
    fn merge_cells(&mut self, winner: CellId, loser: CellId) {
        for slice in self.slices.values_mut() {
            // Read loser cell immutably
            let (lc_conf, lc_tags, lc_meta) = match slice.grid.get_cell(loser) {
                Some(lc) => (lc.confidence, lc.tags.clone(), lc.metadata.clone()),
                None => continue,
            };

            // Mutate winner cell
            if let Some(wc) = slice.grid.get_cell_mut(winner) {
                // Confidence merge
                wc.confidence = wc.confidence.max(lc_conf);

                // Tag merge
                for tag in lc_tags {
                    if !wc.tags.contains(&tag) {
                        wc.tags.push(tag);
                    }
                }

                // Metadata merge (winner keeps its metadata unless empty)
                if wc.metadata.is_none() && lc_meta.is_some() {
                    wc.metadata = lc_meta;
                }

                // Timestamp update
                wc.last_updated = Some(Utc::now());
            }
        }
    }
}






