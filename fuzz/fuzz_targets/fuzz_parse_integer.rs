#![no_main]

use libfuzzer_sys::fuzz_target;
use simd_text::parse_integer;

fuzz_target!(|data: &[u8]| {
    // Only test inputs that look like plausible integer strings:
    // optional leading/trailing whitespace, optional sign, ASCII digits.
    // This keeps the fuzzer focused on the interesting parsing logic
    // rather than spending all its time on clearly-invalid inputs.
    if !is_plausible_integer_input(data) {
        return;
    }

    let ours: Result<i64, _> = parse_integer(data);

    // Build a string for std's parser. parse_integer trims whitespace,
    // so we do the same for a fair comparison.
    let trimmed = trim_ascii(data);
    let as_str = match std::str::from_utf8(trimmed) {
        Ok(s) => s,
        Err(_) => return, // non-UTF-8 after trimming — skip
    };
    let std_result: Result<i64, _> = as_str.parse();

    match (&ours, &std_result) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                a, b,
                "parsed value mismatch: ours={} std={} input={:?}",
                a, b, as_str,
            );
        }
        (Err(_), Err(_)) => {
            // Both agree it is invalid — fine.
        }
        (Ok(v), Err(e)) => {
            panic!(
                "we parsed {} but std returned error {:?} for input {:?}",
                v, e, as_str,
            );
        }
        (Err(e), Ok(v)) => {
            panic!(
                "we returned error {:?} but std parsed {} for input {:?}",
                e, v, as_str,
            );
        }
    }

    // Also fuzz u64 to exercise unsigned path.
    let ours_u: Result<u64, _> = parse_integer(data);
    let std_u: Result<u64, _> = as_str.parse();

    match (&ours_u, &std_u) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                a, b,
                "u64 parsed value mismatch: ours={} std={} input={:?}",
                a, b, as_str,
            );
        }
        (Err(_), Err(_)) => {}
        (Ok(v), Err(e)) => {
            panic!(
                "u64: we parsed {} but std returned error {:?} for input {:?}",
                v, e, as_str,
            );
        }
        (Err(e), Ok(v)) => {
            panic!(
                "u64: we returned error {:?} but std parsed {} for input {:?}",
                e, v, as_str,
            );
        }
    }
});

/// Returns true if every byte is ASCII whitespace, a sign character, or a digit.
fn is_plausible_integer_input(data: &[u8]) -> bool {
    if data.is_empty() {
        return true; // test the empty case
    }
    data.iter().all(|&b| {
        b.is_ascii_digit() || b == b'+' || b == b'-' || b.is_ascii_whitespace()
    })
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map_or(start, |p| p + 1);
    &bytes[start..end]
}
