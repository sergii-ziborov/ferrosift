#![expect(
    missing_docs,
    reason = "criterion_main! generates an undocumented main function"
)]

//! What the library layer costs above the algorithm.
//!
//! For the digests and ciphers, FerroSift *is* RustCrypto — the same code
//! computes the same bytes. Benchmarking those against RustCrypto and
//! reporting a win would be measuring nothing. The question worth asking is
//! the opposite one: how much does going through a recipe cost compared with
//! calling the primitive directly?
//!
//! That difference is the library's own overhead — argument resolution,
//! budget checks, value wrapping — and it is the number that has to stay
//! small, because it is the only part FerroSift is responsible for.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferrosift_bench::{SIZES, engine, recipe, run, sample, text};
use ferrosift_model::Value;
use md5::Digest as _;
use std::hint::black_box;

fn md5_overhead(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe("hash.md5@1", &[]);
    let mut group = criterion.benchmark_group("overhead/md5");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("through-recipe", size),
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
        group.bench_with_input(
            BenchmarkId::new("primitive-direct", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(md5::Md5::digest(black_box(input))));
            },
        );
    }
    group.finish();
}

fn sha256_overhead(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe(
        "hash.sha2@1",
        &[
            ("size", text("256")),
            ("rounds_256", ferrosift_bench::integer(64)),
            ("rounds_512", ferrosift_bench::integer(160)),
        ],
    );
    let mut group = criterion.benchmark_group("overhead/sha256");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("through-recipe", size),
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
        group.bench_with_input(
            BenchmarkId::new("primitive-direct", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(sha2::Sha256::digest(black_box(input))));
            },
        );
    }
    group.finish();
}

/// The floor: an operation that copies its input and does nothing else.
///
/// Everything the recipe layer costs shows up here with no algorithm to hide
/// behind, which makes it the reference point for every other overhead
/// measurement.
///
/// Both entry points are measured, because they are not the same price.
/// `Executor::execute` resolves the recipe against the registry on every
/// call; `CompiledPipeline` resolves once and reuses the result. That
/// difference is the whole reason the compiled path exists, and this is where
/// it is either worth having or is not.
fn identity_floor(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let recipe = recipe("core.identity@1", &[]);
    let compiled = ferrosift::pipeline()
        .step("core.identity@1", ferrosift_model::Arguments::new())
        .compile(&engine)
        .expect("identity pipeline must compile");

    let mut group = criterion.benchmark_group("overhead/identity");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("execute-each-call", size),
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
        group.bench_with_input(
            BenchmarkId::new("compiled-pipeline", size),
            &input,
            |bencher, input| {
                bencher.iter(|| {
                    black_box(
                        compiled
                            .run_bytes(black_box(input))
                            .expect("identity must run"),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, identity_floor, md5_overhead, sha256_overhead);
criterion_main!(benches);
