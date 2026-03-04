//! Number parsing from ASCII bytes.
//!
//! Parse integers and floats directly from byte slices without requiring
//! UTF-8 conversion first.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

mod integer;
mod float;

/// A type that can be parsed as an integer from ASCII bytes.
pub trait Integer: Sized + Copy {
    /// Parse from ASCII bytes.
    fn parse_from_bytes(bytes: &[u8]) -> Result<Self, ParseError>;
}

/// A type that can be parsed as a float from ASCII bytes.
pub trait Float: Sized + Copy {
    /// Parse from ASCII bytes.
    fn parse_from_bytes(bytes: &[u8]) -> Result<Self, ParseError>;
}

/// Error when parsing a number from bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Kind of error.
    pub kind: ParseErrorKind,
    /// Byte position where the error occurred.
    pub position: usize,
}

/// Kind of parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Input is empty.
    Empty,
    /// Found a non-digit character.
    InvalidDigit,
    /// Number is too large for the target type.
    Overflow,
    /// Invalid float format.
    InvalidFloat,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            ParseErrorKind::Empty => write!(f, "empty input"),
            ParseErrorKind::InvalidDigit => write!(f, "invalid digit at position {}", self.position),
            ParseErrorKind::Overflow => write!(f, "overflow at position {}", self.position),
            ParseErrorKind::InvalidFloat => write!(f, "invalid float at position {}", self.position),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

/// Parse an integer from ASCII bytes.
///
/// Handles leading `+`/`-`, leading zeros, and optional whitespace trimming.
///
/// ```
/// use simd_text::parse_integer;
///
/// let value: i32 = parse_integer(b"42").unwrap();
/// assert_eq!(value, 42);
///
/// let value: i64 = parse_integer(b"-1000").unwrap();
/// assert_eq!(value, -1000);
///
/// let value: u32 = parse_integer(b"+123").unwrap();
/// assert_eq!(value, 123);
/// ```
pub fn parse_integer<T: Integer>(bytes: &[u8]) -> Result<T, ParseError> {
    // Trim leading/trailing whitespace
    let trimmed = trim_ascii(bytes);
    T::parse_from_bytes(trimmed)
}

/// Parse a float from ASCII bytes.
///
/// Uses fast-path parsing for common formats.
///
/// ```
/// use simd_text::parse_float;
///
/// let value: f64 = parse_float(b"3.14159").unwrap();
/// assert!((value - 3.14159).abs() < 1e-10);
///
/// let value: f32 = parse_float(b"-0.5").unwrap();
/// assert!((value - (-0.5)).abs() < 1e-6);
/// ```
pub fn parse_float<T: Float>(bytes: &[u8]) -> Result<T, ParseError> {
    let trimmed = trim_ascii(bytes);
    T::parse_from_bytes(trimmed)
}

/// A span in the input where a number was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberSpan {
    /// Byte offset of the start of the number.
    pub offset: usize,
    /// Length in bytes.
    pub len: usize,
    /// Kind of number detected.
    pub kind: NumberKind,
}

/// Kind of number detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    /// An integer (sequence of digits, optional sign).
    Integer,
    /// A float (digits with decimal point and/or exponent).
    Float,
    /// A hexadecimal number (0x prefix).
    Hex,
}

/// Extract all number spans from a text buffer.
///
/// Useful for log analysis: finds integers, floats, and hex numbers.
///
/// ```
/// use simd_text::{extract_numbers, NumberKind};
///
/// let data = b"latency=42ms p99=3.14 code=0xFF";
/// let spans = extract_numbers(data);
/// assert_eq!(spans.len(), 4);
/// assert_eq!(spans[0].kind, NumberKind::Integer); // 42
/// assert_eq!(spans[1].kind, NumberKind::Integer); // 99
/// assert_eq!(spans[2].kind, NumberKind::Float);   // 3.14
/// assert_eq!(spans[3].kind, NumberKind::Hex);     // 0xFF
/// ```
pub fn extract_numbers(data: &[u8]) -> Vec<NumberSpan> {
    let mut spans = Vec::new();
    let len = data.len();
    let mut i = 0;

    while i < len {
        // Skip non-digit, non-sign, non-0x-prefix characters
        if !is_number_start(data, i) {
            i += 1;
            continue;
        }

        // Check for hex: 0x or 0X
        if i + 2 < len && data[i] == b'0' && (data[i + 1] == b'x' || data[i + 1] == b'X') {
            let start = i;
            i += 2;
            while i < len && is_hex_digit(data[i]) {
                i += 1;
            }
            if i > start + 2 {
                spans.push(NumberSpan {
                    offset: start,
                    len: i - start,
                    kind: NumberKind::Hex,
                });
            }
            continue;
        }

        let start = i;
        let mut is_float = false;

        // Optional sign
        if data[i] == b'+' || data[i] == b'-' {
            i += 1;
        }

        // Need at least one digit
        if i >= len || !data[i].is_ascii_digit() {
            i = start + 1;
            continue;
        }

        // Integer part
        while i < len && data[i].is_ascii_digit() {
            i += 1;
        }

        // Decimal point
        if i < len && data[i] == b'.' && i + 1 < len && data[i + 1].is_ascii_digit() {
            is_float = true;
            i += 1;
            while i < len && data[i].is_ascii_digit() {
                i += 1;
            }
        }

        // Exponent
        if i < len && (data[i] == b'e' || data[i] == b'E') {
            let saved = i;
            i += 1;
            if i < len && (data[i] == b'+' || data[i] == b'-') {
                i += 1;
            }
            if i < len && data[i].is_ascii_digit() {
                is_float = true;
                while i < len && data[i].is_ascii_digit() {
                    i += 1;
                }
            } else {
                // Not a valid exponent, backtrack
                i = saved;
            }
        }

        // Don't count a bare sign as a number
        if i > start && !(i == start + 1 && (data[start] == b'+' || data[start] == b'-')) {
            spans.push(NumberSpan {
                offset: start,
                len: i - start,
                kind: if is_float { NumberKind::Float } else { NumberKind::Integer },
            });
        }
    }

    spans
}

fn is_number_start(data: &[u8], i: usize) -> bool {
    let b = data[i];
    b.is_ascii_digit() || ((b == b'+' || b == b'-') && i + 1 < data.len() && data[i + 1].is_ascii_digit())
        || (b == b'0' && i + 1 < data.len() && (data[i + 1] == b'x' || data[i + 1] == b'X'))
}

fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b) || (b'A'..=b'F').contains(&b)
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(bytes.len());
    let end = bytes.iter().rposition(|b| !b.is_ascii_whitespace()).map_or(start, |p| p + 1);
    &bytes[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u32() {
        let v: u32 = parse_integer(b"12345").unwrap();
        assert_eq!(v, 12345);
    }

    #[test]
    fn parse_i32_negative() {
        let v: i32 = parse_integer(b"-42").unwrap();
        assert_eq!(v, -42);
    }

    #[test]
    fn parse_with_plus() {
        let v: i64 = parse_integer(b"+999").unwrap();
        assert_eq!(v, 999);
    }

    #[test]
    fn parse_with_whitespace() {
        let v: u32 = parse_integer(b"  123  ").unwrap();
        assert_eq!(v, 123);
    }

    #[test]
    fn parse_zero() {
        let v: u32 = parse_integer(b"0").unwrap();
        assert_eq!(v, 0);
    }

    #[test]
    fn parse_empty() {
        let err = parse_integer::<u32>(b"").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::Empty);
    }

    #[test]
    fn parse_invalid() {
        let err = parse_integer::<u32>(b"abc").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidDigit);
    }

    #[test]
    fn parse_overflow() {
        let err = parse_integer::<u8>(b"256").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::Overflow);
    }

    #[test]
    fn parse_f64() {
        let v: f64 = parse_float(b"3.14").unwrap();
        assert!((v - 3.14).abs() < 1e-10);
    }

    #[test]
    fn parse_f64_negative() {
        let v: f64 = parse_float(b"-2.5").unwrap();
        assert!((v - (-2.5)).abs() < 1e-10);
    }

    #[test]
    fn parse_f64_exponent() {
        let v: f64 = parse_float(b"1.5e3").unwrap();
        assert!((v - 1500.0).abs() < 1e-10);
    }

    #[test]
    fn parse_f64_integer_form() {
        let v: f64 = parse_float(b"42").unwrap();
        assert!((v - 42.0).abs() < 1e-10);
    }

    #[test]
    fn extract_numbers_mixed() {
        let data = b"temp=23 latency=1.5ms code=0xDEAD count=-3";
        let spans = extract_numbers(data);
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].kind, NumberKind::Integer);
        assert_eq!(&data[spans[0].offset..spans[0].offset + spans[0].len], b"23");
        assert_eq!(spans[1].kind, NumberKind::Float);
        assert_eq!(&data[spans[1].offset..spans[1].offset + spans[1].len], b"1.5");
        assert_eq!(spans[2].kind, NumberKind::Hex);
        assert_eq!(&data[spans[2].offset..spans[2].offset + spans[2].len], b"0xDEAD");
        assert_eq!(spans[3].kind, NumberKind::Integer);
        assert_eq!(&data[spans[3].offset..spans[3].offset + spans[3].len], b"-3");
    }

    #[test]
    fn extract_numbers_empty() {
        assert!(extract_numbers(b"no numbers here").is_empty());
    }

    #[test]
    fn parse_error_display() {
        let err = ParseError { kind: ParseErrorKind::Empty, position: 0 };
        assert!(err.to_string().contains("empty"));

        let err = ParseError { kind: ParseErrorKind::InvalidDigit, position: 3 };
        assert!(err.to_string().contains("position 3"));

        let err = ParseError { kind: ParseErrorKind::Overflow, position: 5 };
        assert!(err.to_string().contains("overflow"));

        let err = ParseError { kind: ParseErrorKind::InvalidFloat, position: 0 };
        assert!(err.to_string().contains("invalid float"));
    }

    #[test]
    fn parse_whitespace_only() {
        let err = parse_integer::<u32>(b"   ").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::Empty);
    }

    #[test]
    fn parse_bare_sign() {
        let err = parse_integer::<i32>(b"+").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidDigit);

        let err = parse_integer::<i32>(b"-").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::InvalidDigit);
    }

    #[test]
    fn extract_numbers_empty_input() {
        assert!(extract_numbers(b"").is_empty());
    }

    #[test]
    fn parse_leading_zeros() {
        let v: u32 = parse_integer(b"007").unwrap();
        assert_eq!(v, 7);
    }
}
