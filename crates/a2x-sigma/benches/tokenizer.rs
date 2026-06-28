// Benchmarks for the a2x-sigma tokenizer.
//
// See PLAN.md §17 (Performance & Benchmarking) and plans/01-sigma-language.md.
//
// Targets:
//   - Tokenizer throughput > 1M packets/sec

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use a2x_sigma::tokenizer::lex;

/// The standard anomaly-scan packet used throughout the codebase tests.
const ANOMALY_SCAN: &str =
    "⟦Σ∞⟧⟬I:⚡✣⩫ ∷ C:⟚⟞⟨sys⟩ ∷ P:⥁⤒⤈ ∷ D:⌮⌳⌱⟭";

fn bench_tokenize_anomaly_scan(c: &mut Criterion) {
    c.bench_function("tokenize_anomaly_scan", |b| {
        b.iter(|| lex(black_box(ANOMALY_SCAN)))
    });
}

fn bench_tokenize_1k_packets(c: &mut Criterion) {
    let input = ANOMALY_SCAN.repeat(1000);
    c.bench_function("tokenize_1k_packets", |b| {
        b.iter(|| lex(black_box(&input)))
    });
}

fn bench_tokenize_10k_packets(c: &mut Criterion) {
    let input = ANOMALY_SCAN.repeat(10_000);
    c.bench_function("tokenize_10k_packets", |b| {
        b.iter(|| lex(black_box(&input)))
    });
}

criterion_group!(
    benches,
    bench_tokenize_anomaly_scan,
    bench_tokenize_1k_packets,
    bench_tokenize_10k_packets,
);
criterion_main!(benches);
