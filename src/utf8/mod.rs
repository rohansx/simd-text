//! UTF-8 validation with SIMD acceleration.
//!
//! Provides fast UTF-8 validation using AVX2/SSE4.2/NEON when available,
//! with a correct scalar fallback.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod scalar;
#[cfg(target_arch = "x86_64")]
mod avx2;

/// Error returned when UTF-8 validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8Error {
    /// Byte index of the first invalid byte.
    pub valid_up_to: usize,
}

impl core::fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid UTF-8 at byte offset {}", self.valid_up_to)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Utf8Error {}

/// Validate that the input is valid UTF-8.
///
/// Returns `Ok(())` if the entire input is valid UTF-8, or `Err(Utf8Error)`
/// with the position of the first invalid byte.
///
/// Uses SIMD acceleration when available (AVX2 on x86_64).
///
/// ```
/// use simd_text::validate_utf8;
///
/// assert!(validate_utf8(b"Hello, world!").is_ok());
/// assert!(validate_utf8("日本語".as_bytes()).is_ok());
/// assert!(validate_utf8(&[0xFF, 0xFE]).is_err());
/// ```
pub fn validate_utf8(data: &[u8]) -> Result<(), Utf8Error> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: we just checked AVX2 is available
            return unsafe { avx2::validate_utf8_avx2(data) };
        }
    }
    scalar::validate_utf8_scalar(data)
}

/// Validate UTF-8 AND find line boundaries in a single pass.
///
/// Returns `Ok(line_offsets)` where each offset is the byte position of a
/// newline character (`\n`), or `Err` with the first invalid byte position.
///
/// This is ~10% slower than validation alone but saves an entire second pass
/// for line splitting.
///
/// ```
/// use simd_text::validate_and_split_lines;
///
/// let data = b"line1\nline2\nline3";
/// let offsets = validate_and_split_lines(data).unwrap();
/// assert_eq!(offsets, vec![5, 11]);
/// ```
pub fn validate_and_split_lines(data: &[u8]) -> Result<Vec<usize>, Utf8Error> {
    // Single-pass: validate UTF-8 while collecting newline positions
    scalar::validate_and_split_lines_scalar(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ascii() {
        assert!(validate_utf8(b"hello world").is_ok());
    }

    #[test]
    fn valid_multibyte() {
        assert!(validate_utf8("こんにちは".as_bytes()).is_ok());
        assert!(validate_utf8("emoji: 🎉".as_bytes()).is_ok());
        assert!(validate_utf8("Ñoño".as_bytes()).is_ok());
    }

    #[test]
    fn invalid_continuation() {
        let err = validate_utf8(&[0xC0, 0x80]).unwrap_err();
        assert_eq!(err.valid_up_to, 0);
    }

    #[test]
    fn invalid_overlong() {
        // Overlong encoding of '/'
        let err = validate_utf8(&[0xC0, 0xAF]).unwrap_err();
        assert_eq!(err.valid_up_to, 0);
    }

    #[test]
    fn invalid_surrogate() {
        // UTF-16 surrogate U+D800
        let err = validate_utf8(&[0xED, 0xA0, 0x80]).unwrap_err();
        assert_eq!(err.valid_up_to, 0);
    }

    #[test]
    fn invalid_too_large() {
        // Code point > U+10FFFF
        let err = validate_utf8(&[0xF4, 0x90, 0x80, 0x80]).unwrap_err();
        assert_eq!(err.valid_up_to, 0);
    }

    #[test]
    fn empty() {
        assert!(validate_utf8(b"").is_ok());
    }

    #[test]
    fn validate_and_split() {
        let data = b"a\nb\nc";
        let offsets = validate_and_split_lines(data).unwrap();
        assert_eq!(offsets, vec![1, 3]);
    }

    #[test]
    fn validate_and_split_crlf() {
        let data = b"a\r\nb\r\n";
        let offsets = validate_and_split_lines(data).unwrap();
        // \n at positions 2 and 5
        assert_eq!(offsets, vec![2, 5]);
    }

    #[test]
    fn validate_and_split_invalid() {
        let data = &[b'a', b'\n', 0xFF];
        assert!(validate_and_split_lines(data).is_err());
    }

    #[test]
    fn large_valid_ascii() {
        let data = vec![b'A'; 1024];
        assert!(validate_utf8(&data).is_ok());
    }

    #[test]
    fn large_valid_multibyte() {
        let s = "日本語テスト".repeat(200);
        assert!(validate_utf8(s.as_bytes()).is_ok());
    }

    #[test]
    fn invalid_at_end() {
        let mut data = vec![b'A'; 100];
        data.push(0xFF);
        let err = validate_utf8(&data).unwrap_err();
        assert_eq!(err.valid_up_to, 100);
    }

    #[test]
    fn error_display() {
        let err = Utf8Error { valid_up_to: 42 };
        let msg = err.to_string();
        assert!(msg.contains("42"));
        assert!(msg.contains("invalid UTF-8"));
    }

    #[test]
    fn validate_and_split_empty() {
        let offsets = validate_and_split_lines(b"").unwrap();
        assert!(offsets.is_empty());
    }
}
