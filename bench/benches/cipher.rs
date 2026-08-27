#![expect(
    missing_docs,
    reason = "criterion_main! generates an undocumented main function"
)]

//! Block cipher throughput, against the reference rather than against a crate.
//!
//! There is no comparison crate here on purpose. TEA and XTEA have Rust
//! implementations, but none of them is wrapped in the reference's five block
//! modes with its padding and its output encoding — measuring a bare block
//! function against a whole operation would compare two different amounts of
//! work and call one of them faster.
//!
//! What the arms *are* comparable with is the reference itself, which does
//! exactly the same work through exactly the same interface.
//! `tools/bench/cyberchef.mjs` bakes the same recipe on the same bytes.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferrosift_bench::{SIZES, compiled, engine, integer, recipe, run, sample, text};
use ferrosift_model::{ArgumentValue, Arguments, Value};
use std::hint::black_box;

/// A `CyberChef` toggleString argument.
fn toggle(option: &str, string: &str) -> ArgumentValue {
    ArgumentValue::Map(Arguments::from([
        ("option".into(), ArgumentValue::Text(option.into())),
        ("string".into(), ArgumentValue::Text(string.into())),
    ]))
}

/// The key and IV every arm uses, so the arms differ only in the cipher.
fn common() -> [(&'static str, ArgumentValue); 6] {
    [
        ("key", toggle("Hex", "00112233445566778899aabbccddeeff")),
        ("iv", toggle("Hex", "0102030405060708")),
        ("mode", text("CBC")),
        ("input", text("Raw")),
        ("output", text("Hex")),
        ("padding", text("PKCS5")),
    ]
}

fn bench_one(criterion: &mut Criterion, group_name: &str, operation: &str, cycles: Option<i128>) {
    let engine = engine().expect("engine");
    let mut arguments: Vec<(&str, ArgumentValue)> = common().into();
    if let Some(cycles) = cycles {
        arguments.push(("rounds", integer(cycles)));
    }
    let recipe = recipe(operation, &arguments);
    let pipeline = compiled(&engine, operation, &arguments);

    let mut group = criterion.benchmark_group(group_name);
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift-per-call", size),
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
            BenchmarkId::new("ferrosift", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(pipeline.run_bytes(black_box(input))));
            },
        );
    }
    group.finish();
}

fn tea_encrypt(criterion: &mut Criterion) {
    bench_one(criterion, "cipher/tea", "crypto.tea.encrypt@1", None);
}

fn xtea_encrypt(criterion: &mut Criterion) {
    bench_one(criterion, "cipher/xtea", "crypto.xtea.encrypt@1", Some(32));
}

criterion_group!(benches, tea_encrypt, xtea_encrypt);
criterion_main!(benches);
