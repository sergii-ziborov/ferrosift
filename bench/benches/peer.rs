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
use ferrosift_bench::{SIZES, boolean, compiled, engine, integer, sample, sample_text, text};
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

/// Runs both sides once and reports whether they agree.
///
/// Printed rather than asserted: a divergence is a finding about the two
/// implementations, not a reason to fail the benchmark run. It *is* a reason
/// not to time them, and the answer says so — an arm whose two sides produce
/// different bytes is not measuring one thing done two ways.
///
/// That gate used to be a comment. The file said a divergence would be
/// reported rather than benchmarked, and then benchmarked it anyway: ROT13
/// diverged on every run and still produced a table, in which rx-chef looked
/// eighty-five times faster because one side was returning an error and being
/// timed doing it.
fn agrees(label: &str, ours: &[u8], theirs: &[u8]) -> bool {
    if ours == theirs {
        println!("agreement: {label} — identical output");
        return true;
    }
    println!(
        "DIVERGENCE: {label} — ferrosift {} bytes, rx-chef {} bytes; not timed",
        ours.len(),
        theirs.len()
    );
    false
}

/// Runs both sides once and says whether timing them would compare anything.
fn comparable(label: &str, ours: &[u8], theirs: Option<&[u8]>) -> bool {
    match theirs {
        Some(theirs) => agrees(label, ours, theirs),
        None => {
            println!("skipping {label}: one side refused the input, so it is not timed");
            false
        }
    }
}

fn base64_encode(criterion: &mut Criterion) {
    let engine = engine().expect("engine");
    let arguments = [("alphabet", text("A-Za-z0-9+/="))];
    let ours = compiled(&engine, "encoding.base64.encode@1", &arguments);

    // Looked up by display name, not by module name: `get_operation` compares
    // against `op.name()` lowercased, so `to_base64` finds nothing while
    // `To Base64` finds the operation. The first version of this file passed
    // the module names and every arm skipped itself silently.
    let theirs = match peer(
        "To Base64",
        vec![rxchef::ArgValue::Str("A-Za-z0-9+/=".into())],
    ) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping base64/encode peer arm: {reason}");
            return;
        }
    };

    let probe = sample(64);
    let Ok(mine) = ours.run_bytes(&probe) else {
        println!("skipping base64/encode: FerroSift refused the probe");
        return;
    };
    if !comparable(
        "base64/encode",
        &mine,
        theirs.run_bytes(probe.clone()).ok().as_deref(),
    ) {
        return;
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
    let theirs = match peer(
        "To Hex",
        vec![
            rxchef::ArgValue::Str("None".into()),
            rxchef::ArgValue::Num(0.0),
        ],
    ) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping hex/encode peer arm: {reason}");
            return;
        }
    };

    let probe = sample(64);
    let Ok(mine) = ours.run_bytes(&probe) else {
        println!("skipping hex/encode: FerroSift refused the probe");
        return;
    };
    if !comparable(
        "hex/encode",
        &mine,
        theirs.run_bytes(probe.clone()).ok().as_deref(),
    ) {
        return;
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

    // Every argument is read with `as_str` and parsed, including the two that
    // are booleans and the one that is a number -- so strings are what the
    // operation actually accepts, and a `Bool` or a `Num` here reads as absent
    // and silently takes the default.
    let theirs = match peer(
        "ROT13",
        vec![
            rxchef::ArgValue::Str("true".into()),
            rxchef::ArgValue::Str("true".into()),
            rxchef::ArgValue::Str("false".into()),
            rxchef::ArgValue::Str("13".into()),
        ],
    ) {
        Ok(pipeline) => pipeline,
        Err(reason) => {
            println!("skipping rot13 peer arm: {reason}");
            return;
        }
    };

    // Text, not random bytes. rx-chef's ROT13 reads its input through a lossy
    // UTF-8 conversion where FerroSift's works on bytes, so random input makes
    // the two do different work -- and on the first run one of them refused it
    // outright and was timed refusing.
    let probe = sample_text(64);
    let Ok(mine) = ours.run_bytes(&probe) else {
        println!("skipping rot13: FerroSift refused the probe");
        return;
    };
    if !comparable(
        "rot13",
        &mine,
        theirs.run_bytes(probe.clone()).ok().as_deref(),
    ) {
        return;
    }

    let mut group = criterion.benchmark_group("peer/rot13");
    for size in SIZES {
        let input = sample_text(size);
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
