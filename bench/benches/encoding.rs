#![expect(
    missing_docs,
    reason = "criterion_main! generates an undocumented main function"
)]

//! Encoding throughput against the specialist crate for each codec.
//!
//! The comparison targets are single-algorithm crates — `base64` and `hex` —
//! chosen because they are the fastest widely used Rust implementations of
//! exactly one thing. FerroSift is measured through a real recipe, so the
//! numbers include the argument resolution and value handling a caller pays
//! for; comparing a raw codec against `base64::encode` would be measuring two
//! different things and calling one of them faster.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferrosift_bench::{SIZES, engine, integer, recipe, run, sample, text};
use ferrosift_model::Value;
use std::hint::black_box;

fn base64_encode(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe(
        "encoding.base64.encode@1",
        &[("alphabet", text("A-Za-z0-9+/="))],
    );
    let mut group = criterion.benchmark_group("base64/encode");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("ferrosift", size), &input, |bencher, input| {
            bencher.iter(|| {
                black_box(run(
                    &engine,
                    &recipe,
                    Value::Bytes(black_box(input).clone()),
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("base64-crate", size), &input, |bencher, input| {
            use base64::Engine as _;
            bencher.iter(|| {
                black_box(base64::engine::general_purpose::STANDARD.encode(black_box(input)))
            });
        });
    }
    group.finish();
}

fn base64_decode(criterion: &mut Criterion) {
    use base64::Engine as _;

    let engine = engine().expect("engine");
    let recipe = recipe(
        "encoding.base64.decode@1",
        &[
            ("alphabet", text("A-Za-z0-9+/=")),
            ("remove_non_alphabet", ferrosift_bench::boolean(true)),
        ],
    );
    let mut group = criterion.benchmark_group("base64/decode");
    for size in SIZES {
        let encoded = base64::engine::general_purpose::STANDARD.encode(sample(size));
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    black_box(run(
                        &engine,
                        &recipe,
                        Value::Text(ferrosift_model::TextValue {
                            text: black_box(encoded).clone(),
                            encoding: ferrosift_model::TextEncoding::Utf8,
                        }),
                    ))
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("base64-crate", size),
            &encoded,
            |bencher, encoded| {
                bencher.iter(|| {
                    black_box(
                        base64::engine::general_purpose::STANDARD
                            .decode(black_box(encoded))
                            .expect("valid base64"),
                    )
                });
            },
        );
    }
    group.finish();
}

fn hex_encode(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    // Both crates emit contiguous lower-case hex, so FerroSift is asked for
    // the same shape: no delimiter and no line wrapping. Anything else would
    // be comparing two different amounts of work.
    let recipe = recipe(
        "encoding.hex.encode@1",
        &[("delimiter", text("None")), ("bytes_per_line", integer(0))],
    );
    let mut group = criterion.benchmark_group("hex/encode");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("ferrosift", size), &input, |bencher, input| {
            bencher.iter(|| {
                black_box(run(
                    &engine,
                    &recipe,
                    Value::Bytes(black_box(input).clone()),
                ))
            });
        });
        group.bench_with_input(BenchmarkId::new("hex-crate", size), &input, |bencher, input| {
            bencher.iter(|| black_box(hex::encode(black_box(input))));
        });
        // `faster-hex` is the SIMD implementation of the same thing. It is
        // here because reporting a win over `hex` while a quicker crate exists
        // would be choosing the opponent.
        group.bench_with_input(
            BenchmarkId::new("faster-hex-crate", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(faster_hex::hex_string(black_box(input))));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, base64_encode, base64_decode, hex_encode);
criterion_main!(benches);
