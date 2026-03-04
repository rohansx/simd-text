//! Float parsing from ASCII bytes.
//!
//! Uses Rust's built-in float parsing (which already uses Eisel-Lemire fast path)
//! after converting bytes to str.

use super::{Float, ParseError, ParseErrorKind};

macro_rules! impl_float {
    ($($ty:ty),*) => {
        $(
            impl Float for $ty {
                fn parse_from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
                    if bytes.is_empty() {
                        return Err(ParseError { kind: ParseErrorKind::Empty, position: 0 });
                    }

                    // Validate that all bytes are ASCII
                    for (i, &b) in bytes.iter().enumerate() {
                        if !b.is_ascii() {
                            return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: i });
                        }
                    }

                    // SAFETY: we just validated all bytes are ASCII, so this is valid UTF-8
                    let s = unsafe { core::str::from_utf8_unchecked(bytes) };

                    s.parse::<$ty>().map_err(|_| ParseError {
                        kind: ParseErrorKind::InvalidFloat,
                        position: 0,
                    })
                }
            }
        )*
    };
}

impl_float!(f32, f64);
