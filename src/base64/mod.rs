//! Base64 encoding and decoding.
//!
//! Standard base64 (RFC 4648) with SIMD acceleration when available.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod scalar;

/// Error when decoding invalid base64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// Byte position of the invalid character.
    pub position: usize,
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid base64 at position {}", self.position)
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
/// The output buffer must be at least `((input.len() + 2) / 3) * 4` bytes.
///
/// ```
/// use simd_text::base64_encode;
///
/// let input = b"Hello, World!";
/// let mut output = vec![0u8; 20];
/// let len = base64_encode(input, &mut output);
/// assert_eq!(&output[..len], b"SGVsbG8sIFdvcmxkIQ==");
/// ```
pub fn base64_encode(input: &[u8], output: &mut [u8]) -> usize {
    scalar::encode_scalar(input, output)
}

/// Decode standard base64 to bytes. Returns number of bytes written.
///
/// The output buffer must be at least `(input.len() / 4) * 3` bytes.
///
/// ```
/// use simd_text::base64_decode;
///
/// let input = b"SGVsbG8sIFdvcmxkIQ==";
/// let mut output = vec![0u8; 15];
/// let len = base64_decode(input, &mut output).unwrap();
/// assert_eq!(&output[..len], b"Hello, World!");
/// ```
pub fn base64_decode(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    scalar::decode_scalar(input, output)
}

/// Calculate the encoded length for a given input length.
pub fn encoded_len(input_len: usize) -> usize {
    ((input_len + 2) / 3) * 4
}

/// Calculate the maximum decoded length for a given encoded length.
pub fn decoded_len(encoded_len: usize) -> usize {
    (encoded_len / 4) * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(input: &[u8]) {
        let enc_len = encoded_len(input.len());
        let mut encoded = vec![0u8; enc_len];
        let elen = base64_encode(input, &mut encoded);
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
            let len = base64_encode(input, &mut output);
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
        assert_eq!(err.position, 0);
    }

    #[test]
    fn roundtrip_large() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        roundtrip(&data);
    }
}
