#![expect(
    missing_docs,
    reason = "criterion_main! generates an undocumented main function"
)]

//! `FerroSift` against `rx-chef`, the other Rust `CyberChef` port.
//!
//! Every other arm in this harness is a specialist: `base64` does one thing,
//! and beating it would mean `FerroSift`'s codec is good. This arm asks a
//! different question — whether a library of *this shape*, with a registry, an
//! operation trait, typed arguments and a pipeline, carries its structure
//! cheaply. Both sides pay that cost here, which is what makes the comparison
//! about the implementations rather than about the architecture.
//!
//! Both are therefore driven the same way: build a pipeline once, then run it
//! per iteration. Reaching past either one to its codec would compare two
//! different amounts of work.
//!
//! Where the two disagree on output, the disagreement is reported rather than
//! benchmarked. A speed comparison between operations that do not produce the
//! same bytes is not a comparison of anything.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferrosift_bench::{SIZES, boolean, compiled, engine, integer, sample, text};
use std::hint::black_box;

/// Builds an `rx-chef` pipeline of one operation, or explains why it could not.
fn peer(
    operation: &str,
    args: Vec<rxchef::ArgValue>,
) -> Result<rxchef::Pipeline, alloc_string::String> {
    let Some(op) = rxchef::operations::get_operation(operation) else {
        return Err(format!("rx-chef has no operation `{operation}`"));
    };
    Ok(rxchef::Pipeline::new().then(op, args))
}

mod alloc_string {
    pub use std::string::String;
}

use alloc_string::String;

/// Runs both sides once and reports whether they agree.
///
/// Printed rather than asserted: a divergence is a finding about the two
/// implementations, not a reason to fail the benchmark run. What it does mean
/// is that the timings below that line are not comparing like with like, and
/// the report says so.
fn compare(label: &str, ours: &[u8], theirs: &[u8]) {
    if ours == theirs {
        println!("agreement: {label} — identical output");
    } else {
        println!(
            "DIVERGENCE: {label} — ferrosift {} bytes, rx-chef {} bytes",
            ours.len(),
            theirs.len()
        );
    }
}

fn base64_encode(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let arguments = [("alphabet", text("A-Za-z0-9+/="))];
    let ours = compiled(&engine, "encoding.base64.encode@1", &arguments);

    let theirs = match peer("to_base64", vec![]) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping base64/encode peer arm: {reason}");
            return;
        }
    };

    let probe = sample(64);
    match (ours.run_bytes(&probe), theirs.run_bytes(probe.clone())) {
        (Ok(a), Ok(b)) => compare("base64/encode", &a, &b),
        _ => println!("skipping base64/encode comparison: one side refused the input"),
    }

    let mut group = criterion.benchmark_group("peer/base64-encode");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(ours.run_bytes(black_box(input))));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rxchef-peer", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(theirs.run_bytes(black_box(input).clone())));
            },
        );
    }
    group.finish();
}

fn hex_encode(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let arguments = [("delimiter", text("None")), ("bytes_per_line", integer(0))];
    let ours = compiled(&engine, "encoding.hex.encode@1", &arguments);

    // rx-chef's To Hex takes a delimiter argument; "None" is the shape that
    // matches what FerroSift is being asked for.
    let theirs = match peer("to_hex", vec![rxchef::ArgValue::String("None".into())]) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping hex/encode peer arm: {reason}");
            return;
        }
    };

    let probe = sample(64);
    match (ours.run_bytes(&probe), theirs.run_bytes(probe.clone())) {
        (Ok(a), Ok(b)) => compare("hex/encode", &a, &b),
        _ => println!("skipping hex/encode comparison: one side refused the input"),
    }

    let mut group = criterion.benchmark_group("peer/hex-encode");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(ours.run_bytes(black_box(input))));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rxchef-peer", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(theirs.run_bytes(black_box(input).clone())));
            },
        );
    }
    group.finish();
}

fn rot13(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let arguments = [
        ("rotate_lower_case_chars", boolean(true)),
        ("rotate_upper_case_chars", boolean(true)),
        ("rotate_numbers", boolean(false)),
        ("amount", integer(13)),
    ];
    let ours = compiled(&engine, "encoding.rot13@1", &arguments);

    let theirs = match peer(
        "rot13",
        vec![
            rxchef::ArgValue::Bool(true),
            rxchef::ArgValue::Bool(true),
            rxchef::ArgValue::Bool(false),
            rxchef::ArgValue::Number(13.0),
        ],
    ) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping rot13 peer arm: {reason}");
            return;
        }
    };

    let probe = sample(64);
    match (ours.run_bytes(&probe), theirs.run_bytes(probe.clone())) {
        (Ok(a), Ok(b)) => compare("rot13", &a, &b),
        _ => println!("skipping rot13 comparison: one side refused the input"),
    }

    let mut group = criterion.benchmark_group("peer/rot13");
    for size in SIZES {
        let input = sample(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::new("ferrosift", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(ours.run_bytes(black_box(input))));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("rxchef-peer", size),
            &input,
            |bencher, input| {
                bencher.iter(|| black_box(theirs.run_bytes(black_box(input).clone())));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, base64_encode, hex_encode, rot13);
criterion_main!(benches);
