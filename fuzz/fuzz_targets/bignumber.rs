//! Arbitrary-precision arithmetic and base conversion.
//!
//! Reached through `DecimalValue::parse`, which is the door a recipe actually
//! comes in by. The base reader was the only entry point here before, and it
//! refuses most of what a JavaScript number literal is allowed to be — so the
//! exponent forms that decide this module's cost, `1e+9999999` and its
//! neighbours, were unreachable from the fuzzer no matter how long it ran.
//!
//! Both operands are the fuzzer's. Arithmetic against a constant one, which is
//! what this used to do, never produces the case that matters: the cost of an
//! addition is set by the *gap* between two exponents, and a gap needs two
//! numbers. `fuzz/seeds/bignumber/` starts the search at that gap rather than
//! waiting for it to be discovered a byte at a time.
//!
//! Three things are being looked for. A panic, which is the usual answer. A
//! round trip through a base that does not land back on the value. And an
//! arithmetic identity that fails — the two that hold whatever the operands
//! are, so a failure is a defect rather than a surprising number.

#![no_main]

use ferrosift_model::DecimalValue;
use ferrosift_operations::jscompat_testing::bignumber;
use libfuzzer_sys::fuzz_target;

/// Every base the reference will read or write.
const BASES: &[u32] = &[
    2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 16, 20, 26, 32, 33, 34, 35, 36,
];

/// Above this, an identity is not checked rather than checked expensively.
///
/// An identity over two ten-million-digit numbers is still an identity, but
/// checking it by rendering both sides spends the whole run on `to_fixed`.
const COMPARABLE: u64 = 1 << 14;

/// What this harness will let one addition cost.
///
/// The same shape of bound the executor applies, and for the same reason:
/// `1e+9999999 + 1e-9999999` is exact, in range, and twenty million digits
/// wide. `math.sum@1` refuses it with `output_limit_exceeded` before computing
/// a digit, using this very predicate; a fuzz target calling the arithmetic
/// directly sits below that guard and has to carry its own, or it spends every
/// run running out of memory on the first seed instead of exploring anything.
///
/// Far lower than the executor's ceiling on purpose. A fuzzer measures inputs
/// per second, and eight thousand digits is already well past the size at
/// which the *shape* of the arithmetic stops changing. The cost is not linear
/// in this number either — a root is Newton's method over the radicand, so
/// twice the digits is rather more than twice the work.
const STEP_CEILING: u64 = 1 << 13;

/// Two values agree, as far as this can be asked cheaply.
///
/// `None` where either side is too large to render: not "they differ", but
/// "not asked". Rendering is the only comparison available that is blind to
/// how a value happens to be stored, and it is also the expensive one.
fn agree(left: &DecimalValue, right: &DecimalValue) -> Option<bool> {
    if left.rendered_len() > COMPARABLE || right.rendered_len() > COMPARABLE {
        return None;
    }
    if left.is_not_a_number() || right.is_not_a_number() {
        // Not a number equals nothing, itself included, so an identity over it
        // says nothing either way.
        return None;
    }
    Some(left.to_fixed() == right.to_fixed())
}

fuzz_target!(|data: &[u8]| {
    let Some((base, rest)) = ferrosift_fuzz::select(BASES, data) else {
        return;
    };
    let Ok(text) = core::str::from_utf8(rest) else {
        return;
    };

    // One operand per line, so a seed file is something a reader can check.
    let (first, second) = text.split_once('\n').unwrap_or((text, ""));
    let left = DecimalValue::parse(first);
    let right = DecimalValue::parse(second);

    // Reading and writing in a base are a pair, so a value the reader accepted
    // is one the writer must be able to spell, and the spelling must read back
    // as the same value. The size guard is before `to_base` rather than after:
    // spelling a ten-million-digit value in base two and then deciding it was
    // too long to compare is paying the whole cost to skip the check.
    if let Some(value) = bignumber::parse_in_base(first, base)
        && value.rendered_len() <= 4096
        && let Some(written) = bignumber::to_base(&value, base)
        && let Some(again) = bignumber::parse_in_base(&written, base)
    {
        assert_eq!(
            bignumber::to_base(&again, base),
            Some(written),
            "{first:?} in base {base} did not survive a round trip"
        );
    }

    // Multiplication cannot amplify: a product's coefficient is the product of
    // two coefficients and its exponent is a sum, so a short pair stays short.
    // Division can, and remainder cannot — it reduces modulo the divisor, and
    // reaches a far exponent by modular exponentiation rather than by writing
    // the digits out.
    for (a, b) in [(&left, &right), (&right, &left)] {
        let _ = bignumber::times(a, b);
        let _ = bignumber::modulo(a, b);
        if bignumber::quotient_min_len(a, b) <= STEP_CEILING {
            let _ = bignumber::divide(a, b);
        }
    }
    for value in [&left, &right] {
        // The root amplifies for the same reason addition does and was not
        // guarded for it: reaching the root of a value at 10^10000000 means
        // building a radicand with ten million digits. That is what this
        // target found on its first run, and `math.stddev@1` now checks
        // `root_min_len` for the same reason this does.
        if bignumber::root_min_len(value) <= STEP_CEILING {
            let _ = bignumber::square_root(value);
        }
        let _ = bignumber::negate(value);
        let _ = bignumber::absolute(value);
    }

    // Addition and subtraction align the two scales, which materialises every
    // digit between them, so they run only where that is affordable — and
    // `sum_min_len` is how the operation decides the same thing.
    let affordable = bignumber::sum_min_len(&left, &right) <= STEP_CEILING;
    if affordable {
        for (a, b) in [(&left, &right), (&right, &left)] {
            let _ = bignumber::plus(a, b);
            let _ = bignumber::minus(a, b);
        }
    }

    // Addition and multiplication do not care which operand came first. These
    // are identities that hold for every pair the model can hold, so a
    // counterexample is a defect and not an edge case being discovered.
    if affordable
        && let Some(same) = agree(
            &bignumber::plus(&left, &right),
            &bignumber::plus(&right, &left),
        )
    {
        assert!(same, "addition of {first:?} and {second:?} depends on order");
    }
    if let Some(same) = agree(
        &bignumber::times(&left, &right),
        &bignumber::times(&right, &left),
    ) {
        assert!(
            same,
            "multiplication of {first:?} and {second:?} depends on order"
        );
    }

    // And negation is its own inverse, which catches a sign that survives one
    // application and not two.
    if let Some(same) = agree(&bignumber::negate(&bignumber::negate(&left)), &left) {
        assert!(same, "negating {first:?} twice did not return it");
    }
});
