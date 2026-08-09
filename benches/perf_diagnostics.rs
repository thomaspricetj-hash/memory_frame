// benches/perf_diagnostics.rs
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memory_frame::frame::memory_frame::MemoryFrame;
use memory_frame::frame::{SliceData, CrossConnect, CellId};
use memory_frame::layers::LayerId;
use memory_frame::config::defaults::default_policy;
use memory_frame::frame::adaptive_rules::AdaptiveRuleEngine;

/// Helper to populate a frame with `n` slices of default grid size.
fn populate_frame_with_slices(n: usize, policy: memory_frame::config::MemoryPolicy) -> MemoryFrame {
    let mut frame = MemoryFrame::new(policy);
    for i in 0..n {
        let id = LayerId::Visual; // reuse same layer id for simplicity
        let data = SliceData::Visual(vec![i as u8]);
        frame.insert_slice(id, data);
    }
    frame
}

fn bench_insert_slice(c: &mut Criterion) {
    let policy = default_policy();
    let mut group = c.benchmark_group("insert_slice");
    for &size in &[1usize, 10, 50, 200] {
        let mut frame = populate_frame_with_slices(size, policy.clone());
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &_s| {
            b.iter(|| {
                let id = LayerId::Visual;
                let data = SliceData::Visual(vec![0u8]);
                frame.insert_slice(id, data);
            })
        });
    }
    group.finish();
}

fn bench_auto_adapt(c: &mut Criterion) {
    let policy = default_policy();
    let mut group = c.benchmark_group("auto_adapt");
    for &size in &[1usize, 10, 50, 200, 500] {
        let mut frame = populate_frame_with_slices(size, policy.clone());
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &_s| {
            b.iter(|| {
                // call the internal auto_adapt hook via a public method if available,
                // otherwise call recompute + adaptive engine sequence.
                frame.recompute_diagonal_metrics();
                frame.recompute_temporal_metrics();
                let mut engine = AdaptiveRuleEngine::default();
                let _ = engine.score_frame_total(&frame, &policy);
            })
        });
    }
    group.finish();
}

fn bench_recompute_diagonal(c: &mut Criterion) {
    let policy = default_policy();
    let mut group = c.benchmark_group("recompute_diagonal_metrics");
    for &size in &[1usize, 10, 50, 200, 500] {
        let mut frame = populate_frame_with_slices(size, policy.clone());
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &_s| {
            b.iter(|| {
                frame.recompute_diagonal_metrics();
            })
        });
    }
    group.finish();
}

fn bench_cross_connect_traverse(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_connect_traverse");
    // build a cross connect with many links
    let mut cc = CrossConnect::new();
    // create a chain of 100 nodes
    for i in 0..100usize {
        let a = CellId { x: i, y: 0 };
        let b = CellId { x: i + 1, y: 0 };
        cc.add_link(a, b, 1.0);
    }
    group.bench_function("traverse_2_hops", |b| {
        b.iter(|| {
            let _ = cc.traverse_multi_hop(CellId { x: 0, y: 0 }, 2, 0.1);
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_insert_slice,
    bench_auto_adapt,
    bench_recompute_diagonal,
    bench_cross_connect_traverse
);
criterion_main!(benches);

