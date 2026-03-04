//! # simd-text
//!
//! Unified SIMD text processing toolkit for Rust.
//!
//! One crate for UTF-8 validation, line splitting, field extraction, number parsing,
//! character classification, and base64 — all with consistent SIMD detection, one API
//! style, and composable pipeline support.
//!
//! ## Quick Start
//!
//! ```
//! use simd_text::{detect, validate_utf8, line_ranges, CharClassifier};
//!
//! // Check available SIMD level
//! let level = detect();
//! println!("SIMD level: {:?}", level);
//!
//! // Validate UTF-8
//! let data = b"Hello, world!\nSecond line\n";
//! assert!(validate_utf8(data).is_ok());
//!
//! // Split into lines
//! let lines: Vec<_> = line_ranges(data).collect();
//! assert_eq!(lines.len(), 2);
//! assert_eq!(&data[lines[0].0..lines[0].1], b"Hello, world!");
//!
//! // Classify characters
//! let classifier = CharClassifier::new(b",!\n");
//! let positions = classifier.find_all(data);
//! assert!(!positions.is_empty());
//! ```
//!
//! ## Pipeline API
//!
//! Fuse multiple operations into a single pass for maximum throughput:
//!
//! ```
//! use simd_text::pipeline;
//!
//! let data = b"line1\nline2\nline3\n";
//! let pipe = pipeline()
//!     .validate_utf8()
//!     .split_lines()
//!     .build();
//!
//! let results = pipe.process(data);
//! assert!(results.utf8_valid);
//! assert_eq!(results.lines.len(), 3);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};

mod detect_mod;
pub mod utf8;
pub mod lines;
pub mod classify;
pub mod fields;
pub mod numbers;
pub mod base64;
mod pipeline_mod;
mod aligned;

// Re-exports
pub use detect_mod::{detect, SimdLevel};
pub use utf8::{validate_utf8, validate_and_split_lines, Utf8Error};
pub use lines::{line_ranges, LineRanges, LineScanner};
pub use classify::CharClassifier;
pub use fields::{split_fields, split_records, Fields, Records};
pub use numbers::{
    parse_integer, parse_float, extract_numbers, Integer, Float,
    NumberSpan, NumberKind, ParseError, ParseErrorKind,
};
pub use base64::{base64_encode, base64_decode, DecodeError};
pub use pipeline_mod::{pipeline, PipelineBuilder, Pipeline, PipelineResults};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_valid_level() {
        let level = detect();
        // Should always return something
        match level {
            SimdLevel::Scalar | SimdLevel::Sse42 | SimdLevel::Avx2
            | SimdLevel::Neon | SimdLevel::Wasm128 => {}
        }
    }

    #[test]
    fn end_to_end_csv_like() {
        let data = b"name,age,city\nAlice,30,NYC\nBob,25,LA\n";

        // Validate
        assert!(validate_utf8(data).is_ok());

        // Split lines
        let lines: Vec<_> = line_ranges(data).collect();
        assert_eq!(lines.len(), 3);

        // Extract fields from second line
        let line = &data[lines[1].0..lines[1].1];
        let fields: Vec<_> = split_fields(line, b',').collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], b"Alice");
        assert_eq!(fields[1], b"30");
        assert_eq!(fields[2], b"NYC");

        // Parse the age
        let age: u32 = parse_integer(fields[1]).unwrap();
        assert_eq!(age, 30);
    }

    #[test]
    fn end_to_end_base64_roundtrip() {
        let original = b"Hello, SIMD world! 1234567890";
        let mut encoded = vec![0u8; original.len() * 2];
        let enc_len = base64_encode(original, &mut encoded);
        encoded.truncate(enc_len);

        let mut decoded = vec![0u8; encoded.len()];
        let dec_len = base64_decode(&encoded, &mut decoded).unwrap();
        decoded.truncate(dec_len);

        assert_eq!(&decoded, original);
    }
}
