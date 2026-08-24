#![expect(
    missing_docs,
    reason = "criterion_main! generates an undocumented main function"
)]

//! Checksums and distances against their specialist crates.
//!
//! These are the families where `FerroSift` wrote the algorithm itself rather
//! than wrapping `RustCrypto`, so a comparison against an independent
//! implementation says something. `crc32fast` uses SIMD where the CPU has it;
//! `strsim` is the usual Rust edit-distance crate.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferrosift_bench::{SIZES, engine, recipe, run, sample, text};
use ferrosift_model::{TextEncoding, TextValue, Value};
use std::hint::black_box;

fn adler32(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe("checksum.adler32@1", &[]);
    let mut group = criterion.benchmark_group("checksum/adler32");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &input,
            |bencher, input| {
                bencher.iter(|| {
                    black_box(run(
                        &engine,
                        &recipe,
                        Value::Bytes(black_box(input).clone()),
                    ))
                });
            },
        );
    }
    group.finish();
}

fn crc32(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("checksum/crc32");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        // No FerroSift arm: `CRC Checksum` is not implemented yet. The
        // reference number is recorded now so the comparison is ready the day
        // it is, rather than being chosen after the result is known.
        group.bench_with_input(
            BenchmarkId::new("crc32fast-crate", size),
            &input,
            |bencher, input| {
                bencher.iter(|| {
                    let mut hasher = crc32fast::Hasher::new();
                    hasher.update(black_box(input));
                    black_box(hasher.finalize())
                });
            },
        );
    }
    group.finish();
}

fn levenshtein(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe(
        "distance.levenshtein@1",
        &[
            ("delimiter", text("\n")),
            ("insertion_cost", ferrosift_bench::integer(1)),
            ("deletion_cost", ferrosift_bench::integer(1)),
            ("substitution_cost", ferrosift_bench::integer(1)),
        ],
    );
    let mut group = criterion.benchmark_group("distance/levenshtein");
    // Edit distance is quadratic, so the sweep stops well short of the byte
    // sizes the linear operations use.
    for size in [16usize, 64, 256, 1024] {
        let left: String = sample(size)
            .iter()
            .map(|byte| char::from(byte % 26 + b'a'))
            .collect();
        let right: String = sample(size)
            .iter()
            .map(|byte| char::from(byte % 25 + b'a'))
            .collect();
        let joined = alloc_joined(&left, &right);
        group.throughput(Throughput::Elements((size * size) as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &joined,
            |bencher, joined| {
                bencher.iter(|| {
                    black_box(run(
                        &engine,
                        &recipe,
                        Value::Text(TextValue {
                            text: black_box(joined).clone(),
                            encoding: TextEncoding::Utf8,
                        }),
                    ))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("strsim-crate", size),
            &(left.clone(), right.clone()),
            |bencher, (left, right)| {
                bencher.iter(|| black_box(strsim::levenshtein(black_box(left), black_box(right))));
            },
        );
    }
    group.finish();
}

fn alloc_joined(left: &str, right: &str) -> String {
    let mut joined = String::with_capacity(left.len() + right.len() + 1);
    joined.push_str(left);
    joined.push('\n');
    joined.push_str(right);
    joined
}

criterion_group!(benches, adler32, crc32, levenshtein);
criterion_main!(benches);
