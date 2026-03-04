#![no_main]

use libfuzzer_sys::fuzz_target;
use simd_text::CharClassifier;

/// The fixed set of characters to classify.
/// Covers common delimiters: space, comma, newline, tab, colon, semicolon.
const CLASSIFY_CHARS: &[u8] = b" ,\n\t:;";

fuzz_target!(|data: &[u8]| {
    let classifier = CharClassifier::new(CLASSIFY_CHARS);
    let simd_positions = classifier.find_all(data);

    // Naive reference: linear scan for matching bytes.
    let naive_positions: Vec<usize> = data
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| {
            if CLASSIFY_CHARS.contains(&b) {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        simd_positions.len(),
        naive_positions.len(),
        "position count mismatch: simd={} naive={} on input of len {}\nsimd:  {:?}\nnaive: {:?}",
        simd_positions.len(),
        naive_positions.len(),
        data.len(),
        &simd_positions[..simd_positions.len().min(20)],
        &naive_positions[..naive_positions.len().min(20)],
    );

    assert_eq!(
        simd_positions, naive_positions,
        "position mismatch on input of len {}",
        data.len(),
    );
});
