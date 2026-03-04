#![no_main]

use libfuzzer_sys::fuzz_target;
use simd_text::line_ranges;

fuzz_target!(|data: &[u8]| {
    // Collect the lines produced by simd-text's line_ranges.
    let simd_lines: Vec<(usize, usize)> = line_ranges(data).collect();

    // Build the reference using a naive newline split.
    // line_ranges splits on \n and strips \r from CRLF endings.
    // It does NOT emit a trailing empty line after a final \n.
    let reference_lines = naive_line_ranges(data);

    assert_eq!(
        simd_lines.len(),
        reference_lines.len(),
        "line count mismatch: simd={} naive={} on input of len {}\nsimd:  {:?}\nnaive: {:?}",
        simd_lines.len(),
        reference_lines.len(),
        data.len(),
        simd_lines,
        reference_lines,
    );

    for (i, (simd_range, naive_range)) in simd_lines.iter().zip(reference_lines.iter()).enumerate()
    {
        let simd_content = &data[simd_range.0..simd_range.1];
        let naive_content = &data[naive_range.0..naive_range.1];
        assert_eq!(
            simd_content, naive_content,
            "line {} content mismatch:\n  simd  [{}, {}) = {:?}\n  naive [{}, {}) = {:?}\n  input len = {}",
            i,
            simd_range.0, simd_range.1, simd_content,
            naive_range.0, naive_range.1, naive_content,
            data.len(),
        );
    }
});

/// Naive reference implementation of line splitting that matches the contract
/// of `line_ranges`:
///   - Split on `\n`.
///   - Strip trailing `\r` from each line (CRLF support).
///   - Trailing content without a final `\n` is included as the last line.
///   - A final `\n` does NOT produce a trailing empty line.
fn naive_line_ranges(data: &[u8]) -> Vec<(usize, usize)> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut start = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            // Strip \r if CRLF
            let end = if i > start && data[i - 1] == b'\r' {
                i - 1
            } else {
                i
            };
            lines.push((start, end));
            start = i + 1;
        }
    }

    // Trailing content without a final newline
    if start < data.len() {
        lines.push((start, data.len()));
    }

    lines
}
