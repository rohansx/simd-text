//! Base64 encoding and decoding.
//!
//! Standard base64 (RFC 4648) with SIMD acceleration when available.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod scalar;

/// Error when decoding invalid base64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Found an invalid character at the given position.
    InvalidByte {
        /// Byte position of the invalid character.
        position: usize,
    },
    /// Input length is not valid (not a multiple of 4 and cannot be
    /// interpreted as unpadded base64).
    InvalidLength {
        /// The length of the input.
        length: usize,
    },
    /// Output buffer is too small for the decoded data.
    OutputTooSmall {
        /// Minimum required output buffer size.
        needed: usize,
        /// Actual output buffer size provided.
        actual: usize,
    },
}

impl DecodeError {
    /// Return the byte position of the error, if applicable.
    pub fn position(&self) -> Option<usize> {
        match self {
            DecodeError::InvalidByte { position } => Some(*position),
            _ => None,
        }
    }
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::InvalidByte { position } => {
                write!(f, "invalid base64 byte at position {}", position)
            }
            DecodeError::InvalidLength { length } => {
                write!(f, "invalid base64 input length {}", length)
            }
            DecodeError::OutputTooSmall { needed, actual } => {
                write!(
                    f,
                    "output buffer too small: need at least {} bytes, got {}",
                    needed, actual
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

/// Standard base64 alphabet (RFC 4648).
const ENCODE_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Decode table: maps ASCII byte -> 6-bit value (0xFF = invalid).
const DECODE_TABLE: [u8; 256] = {
    let mut table = [0xFFu8; 256];
    let mut i = 0u8;
    loop {
        if i >= 64 {
            break;
        }
        table[ENCODE_TABLE[i as usize] as usize] = i;
        i += 1;
    }
    // Padding
    table[b'=' as usize] = 0xFE;
    table
};

/// Encode bytes to standard base64. Returns number of bytes written.
///
/// Returns `Err(DecodeError::OutputTooSmall)` if the output buffer is
/// smaller than `encoded_len(input.len())`.
///
/// ```
/// use simd_text::base64_encode;
///
/// let input = b"Hello, World!";
/// let mut output = vec![0u8; 20];
/// let len = base64_encode(input, &mut output).unwrap();
/// assert_eq!(&output[..len], b"SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn base64_encode(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    let needed = encoded_len(input.len());
    if output.len() < needed {
        return Err(DecodeError::OutputTooSmall {
            needed,
            actual: output.len(),
        });
    }
    Ok(scalar::encode_scalar(input, output))
}

/// Decode standard base64 to bytes. Returns number of bytes written.
///
/// Accepts both padded and unpadded input. The output buffer must be at
/// least `(input.len() / 4) * 3` bytes (or `((input.len() + 3) / 4) * 3`
/// for unpadded input).
///
/// ```
/// use simd_text::base64_decode;
///
/// // Padded input
/// let input = b"SGVsbG8sIFdvcmxkIQ==";
/// let mut output = vec![0u8; 15];
/// let len = base64_decode(input, &mut output).unwrap();
/// assert_eq!(&output[..len], b"Hello, World!");
///
/// // Unpadded input also works
/// let input = b"SGVsbG8";
/// let mut output = vec![0u8; 6];
/// let len = base64_decode(input, &mut output).unwrap();
/// assert_eq!(&output[..len], b"Hello");
/// ```
pub fn base64_decode(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    scalar::decode_scalar(input, output)
}

/// Calculate the encoded length for a given input length.
pub fn encoded_len(input_len: usize) -> usize {
    ((input_len + 2) / 3) * 4
}

/// Calculate the maximum decoded length for a given encoded length.
///
/// Handles both padded (multiple of 4) and unpadded input lengths.
/// For padded input, the actual decoded length may be less if the
/// input ends with `=` padding characters.
pub fn decoded_len(encoded_len: usize) -> usize {
    let full_groups = encoded_len / 4;
    let remainder = encoded_len % 4;
    full_groups * 3 + match remainder {
        2 => 1,
        3 => 2,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(input: &[u8]) {
        let enc_len = encoded_len(input.len());
        let mut encoded = vec![0u8; enc_len];
        let elen = base64_encode(input, &mut encoded).unwrap();
        encoded.truncate(elen);

        let dec_len = decoded_len(elen);
        let mut decoded = vec![0u8; dec_len];
        let dlen = base64_decode(&encoded, &mut decoded).unwrap();
        decoded.truncate(dlen);

        assert_eq!(&decoded, input, "roundtrip failed for input of len {}", input.len());
    }

    #[test]
    fn roundtrip_empty() {
        roundtrip(b"");
    }

    #[test]
    fn roundtrip_one() {
        roundtrip(b"a");
    }

    #[test]
    fn roundtrip_two() {
        roundtrip(b"ab");
    }

    #[test]
    fn roundtrip_three() {
        roundtrip(b"abc");
    }

    #[test]
    fn roundtrip_hello() {
        roundtrip(b"Hello, World!");
    }

    #[test]
    fn roundtrip_binary() {
        let data: Vec<u8> = (0..=255).collect();
        roundtrip(&data);
    }

    #[test]
    fn encode_known_vectors() {
        let cases = [
            (b"" as &[u8], ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ];

        for (input, expected) in cases {
            let mut output = vec![0u8; encoded_len(input.len())];
            let len = base64_encode(input, &mut output).unwrap();
            assert_eq!(
                core::str::from_utf8(&output[..len]).unwrap(),
                expected,
                "encode failed for {:?}",
                core::str::from_utf8(input).unwrap()
            );
        }
    }

    #[test]
    fn decode_invalid() {
        let err = base64_decode(b"!!!!", &mut [0u8; 4]).unwrap_err();
        assert_eq!(err, DecodeError::InvalidByte { position: 0 });
    }

    #[test]
    fn roundtrip_large() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        roundtrip(&data);
    }

    #[test]
    fn encode_output_too_small() {
        let input = b"Hello";
        let mut output = [0u8; 2]; // too small
        let err = base64_encode(input, &mut output).unwrap_err();
        assert!(matches!(err, DecodeError::OutputTooSmall { .. }));
    }

    #[test]
    fn decode_output_too_small() {
        let input = b"SGVsbG8="; // decodes to "Hello" (5 bytes), needs at least 6 from formula
        let mut output = [0u8; 1]; // too small
        let err = base64_decode(input, &mut output).unwrap_err();
        assert!(matches!(err, DecodeError::OutputTooSmall { .. }));
    }

    #[test]
    fn decode_unpadded_2chars() {
        // "Zg==" encodes "f", unpadded form is "Zg"
        let mut output = [0u8; 3];
        let len = base64_decode(b"Zg", &mut output).unwrap();
        assert_eq!(&output[..len], b"f");
    }

    #[test]
    fn decode_unpadded_3chars() {
        // "Zm8=" encodes "fo", unpadded form is "Zm8"
        let mut output = [0u8; 3];
        let len = base64_decode(b"Zm8", &mut output).unwrap();
        assert_eq!(&output[..len], b"fo");
    }

    #[test]
    fn decode_invalid_length_1() {
        // A single base64 character is never valid (not enough bits)
        let err = base64_decode(b"Z", &mut [0u8; 3]).unwrap_err();
        assert!(matches!(err, DecodeError::InvalidLength { .. }));
    }

    #[test]
    fn decode_error_display() {
        let err = DecodeError::InvalidByte { position: 5 };
        let msg = err.to_string();
        assert!(msg.contains("position 5"));

        let err = DecodeError::InvalidLength { length: 7 };
        let msg = err.to_string();
        assert!(msg.contains("length 7"));

        let err = DecodeError::OutputTooSmall { needed: 10, actual: 5 };
        let msg = err.to_string();
        assert!(msg.contains("10") && msg.contains("5"));
    }
}
