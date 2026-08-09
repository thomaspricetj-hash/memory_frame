// tests/perf_max_load.rs
// Full performance smoke harness for "max load" measurements.
// Run with: cargo test --release -- --nocapture
//
// Prints mean and stddev timings (ms) for auto_adapt-like sequence
// across a range of frame sizes and a cross-connect traversal test.
//
// NOTE: This is a smoke/diagnostic test. Run in release mode for meaningful numbers.

use memory_frame::frame::memory_frame::MemoryFrame;
use memory_frame::frame::{SliceData, CrossConnect, CellId};
use memory_frame::layers::LayerId;
use memory_frame::config::defaults::default_policy;
use memory_frame::frame::adaptive_rules::AdaptiveRuleEngine;
use std::time::Instant;

/// Populate a frame with `n` slices. Each slice is slightly varied:
/// - some cells get higher confidence
/// - some cells get tags
///
/// This simulates a realistic, heterogeneous workload.
fn populate_frame_with_variation(
    n: usize,
    policy: memory_frame::config::MemoryPolicy,
) -> MemoryFrame {
    let mut frame = MemoryFrame::new(policy);

    for i in 0..n {
        let id = match i % 6 {
            0 => LayerId::Visual,
            1 => LayerId::Semantic,
            2 => LayerId::Declarative,
            3 => LayerId::Temporal,
            4 => LayerId::Emotional,
            // Use Visual as a safe default when no explicit "Other" variant exists.
            _ => LayerId::Visual,
        };

        // Small payload per slice; content not important for timing but we vary it.
        let data = match id {
            LayerId::Visual => SliceData::Visual(vec![((i % 255) as u8)]),
            LayerId::Semantic => {
                let json = serde_json::json!({ "idx": i, "topic": format!("t{}", i % 10) });
                SliceData::Semantic(json)
            }
            LayerId::Declarative => SliceData::Declarative(format!("fact-{}", i)),
            LayerId::Temporal => SliceData::Temporal(chrono::Utc::now()),
            LayerId::Emotional => SliceData::Emotional(((i % 100) as f32) / 100.0),
            _ => SliceData::Visual(vec![0u8]),
        };

        frame.insert_slice(id, data);

        // Light-weight per-slice mutation to create variance:
        // touch a few cells to set confidence and tags.
        if let Some(slice) = frame.slices.values_mut().last() {
            // mutate up to 3 cells per slice (cheap)
            for k in 0..3 {
                let x = (k * 7 + i) % slice.grid.width;
                let y = (k * 11 + i) % slice.grid.height;
                let cid = CellId { x, y };
                if let Some(cell) = slice.grid.get_cell_mut(cid) {
                    // vary confidence and tags deterministically
                    cell.confidence = (((i + k) % 100) as f32) / 100.0;
                    if (i + k) % 5 == 0 {
                        cell.add_tag("hot");
                    }
                    if (i + k) % 7 == 0 {
                        cell.add_tag("freq");
                    }
                    cell.last_updated = Some(chrono::Utc::now());
                }
            }
        }
    }

    frame
}

/// Compute mean and stddev (ms) from a vector of nanosecond samples.
fn stats_ms(samples_ns: &[f64]) -> (f64, f64) {
    let n = samples_ns.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let mean_ns = samples_ns.iter().sum::<f64>() / n;
    let var_ns = samples_ns
        .iter()
        .map(|v| {
            let d = v - mean_ns;
            d * d
        })
        .sum::<f64>()
        / n;
    let std_ns = var_ns.sqrt();
    (mean_ns / 1_000_000.0, std_ns / 1_000_000.0)
}

#[test]
fn perf_max_load_auto_adapt() {
    // Sizes to exercise: small -> medium -> large -> very large
    // Reduced defaults for CI and local machines; increase for dedicated benchmarking.
    let sizes = [100usize, 500, 1_000, 2_000];

    // Number of repeated runs per size to compute mean/stddev
    let runs = 6usize;

    let policy = default_policy();

    println!("=== PERF MAX LOAD: auto_adapt-like sequence ===");
    for &n in &sizes {
        // Populate frame (this itself is part of the workload; we measure adapt only)
        println!("Preparing frame with {} slices (this may take a while)...", n);
        let mut frame = populate_frame_with_variation(n, policy.clone());

        // Warm up a single run to stabilize caches
        {
            frame.recompute_diagonal_metrics();
            frame.recompute_temporal_metrics();
            let mut engine = AdaptiveRuleEngine::default();
            let _ = engine.score_frame_total(&frame, &policy);
        }

        // Collect timings (ns)
        let mut samples_ns: Vec<f64> = Vec::with_capacity(runs);
        for _ in 0..runs {
            let t0 = Instant::now();
            frame.recompute_diagonal_metrics();
            frame.recompute_temporal_metrics();
            let mut engine = AdaptiveRuleEngine::default();
            let _score = engine.score_frame_total(&frame, &policy);
            let elapsed_ns = t0.elapsed().as_nanos() as f64;
            samples_ns.push(elapsed_ns);
        }

        let (mean_ms, std_ms) = stats_ms(&samples_ns);
        println!(
            "slices={:>6}  mean_ms={:>8.3}  stddev_ms={:>7.3}  runs={}",
            n, mean_ms, std_ms, runs
        );
    }
}

/// Cross-connect heavy traversal test: builds a dense graph and measures multi-hop traversal.
#[test]
fn perf_max_load_cross_connect() {
    // Node count and average out-degree to simulate heavy graph
    // Reduced defaults for CI; increase for heavier local benchmarking.
    let node_count = 1_000usize;
    let avg_out = 6usize;
    let hops = [1usize, 2, 3, 5];

    println!("=== PERF MAX LOAD: cross-connect traversal ===");
    let mut cc = CrossConnect::new();

    // Build a pseudo-random-ish graph deterministically
    for i in 0..node_count {
        for j in 1..=avg_out {
            let a = CellId {
                x: i,
                y: 0,
            };
            let b = CellId {
                x: (i + j) % node_count,
                y: 0,
            };
            // weight varies slightly
            let w = 0.5 + ((i + j) % 10) as f32 / 20.0;
            cc.add_link(a, b, w);
        }
    }

    // Warm up
    let _ = cc.traverse_multi_hop(CellId { x: 0, y: 0 }, 2, 0.05);

    for &h in &hops {
        let runs = 6usize;
        let mut samples_ns: Vec<f64> = Vec::with_capacity(runs);
        for _ in 0..runs {
            let t0 = Instant::now();
            let _res = cc.traverse_multi_hop(CellId { x: 0, y: 0 }, h, 0.05);
            let elapsed_ns = t0.elapsed().as_nanos() as f64;
            samples_ns.push(elapsed_ns);
        }
        let (mean_ms, std_ms) = stats_ms(&samples_ns);
        println!(
            "traverse hops={}  mean_ms={:.3}  stddev_ms={:.3}  runs={}",
            h, mean_ms, std_ms, runs
        );
    }
}

