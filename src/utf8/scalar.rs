//! Scalar UTF-8 validation — byte-by-byte fallback.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use super::Utf8Error;

/// Validate UTF-8 byte-by-byte. Always correct, used as fallback.
pub(crate) fn validate_utf8_scalar(data: &[u8]) -> Result<(), Utf8Error> {
    let len = data.len();
    let mut i = 0;

    while i < len {
        let b = data[i];

        if b < 0x80 {
            // ASCII: single byte
            i += 1;
        } else if b < 0xC2 {
            // Invalid: overlong or continuation byte as start
            return Err(Utf8Error { valid_up_to: i });
        } else if b < 0xE0 {
            // 2-byte sequence: 110xxxxx 10xxxxxx
            if i + 1 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            if data[i + 1] & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            i += 2;
        } else if b < 0xF0 {
            // 3-byte sequence: 1110xxxx 10xxxxxx 10xxxxxx
            if i + 2 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            let b1 = data[i + 1];
            let b2 = data[i + 2];
            if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            // Check overlong and surrogates
            match b {
                0xE0 if b1 < 0xA0 => return Err(Utf8Error { valid_up_to: i }),
                0xED if b1 > 0x9F => return Err(Utf8Error { valid_up_to: i }),
                _ => {}
            }
            i += 3;
        } else if b < 0xF5 {
            // 4-byte sequence: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
            if i + 3 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            let b1 = data[i + 1];
            let b2 = data[i + 2];
            let b3 = data[i + 3];
            if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 || b3 & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            // Check overlong and out-of-range
            match b {
                0xF0 if b1 < 0x90 => return Err(Utf8Error { valid_up_to: i }),
                0xF4 if b1 > 0x8F => return Err(Utf8Error { valid_up_to: i }),
                _ => {}
            }
            i += 4;
        } else {
            return Err(Utf8Error { valid_up_to: i });
        }
    }

    Ok(())
}

/// Validate UTF-8 and collect newline positions in a single pass.
pub(crate) fn validate_and_split_lines_scalar(data: &[u8]) -> Result<Vec<usize>, Utf8Error> {
    let len = data.len();
    let mut i = 0;
    let mut newlines = Vec::new();

    while i < len {
        let b = data[i];

        if b < 0x80 {
            if b == b'\n' {
                newlines.push(i);
            }
            i += 1;
        } else if b < 0xC2 {
            return Err(Utf8Error { valid_up_to: i });
        } else if b < 0xE0 {
            if i + 1 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            if data[i + 1] & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            i += 2;
        } else if b < 0xF0 {
            if i + 2 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            let b1 = data[i + 1];
            let b2 = data[i + 2];
            if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            match b {
                0xE0 if b1 < 0xA0 => return Err(Utf8Error { valid_up_to: i }),
                0xED if b1 > 0x9F => return Err(Utf8Error { valid_up_to: i }),
                _ => {}
            }
            i += 3;
        } else if b < 0xF5 {
            if i + 3 >= len {
                return Err(Utf8Error { valid_up_to: i });
            }
            let b1 = data[i + 1];
            let b2 = data[i + 2];
            let b3 = data[i + 3];
            if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 || b3 & 0xC0 != 0x80 {
                return Err(Utf8Error { valid_up_to: i });
            }
            match b {
                0xF0 if b1 < 0x90 => return Err(Utf8Error { valid_up_to: i }),
                0xF4 if b1 > 0x8F => return Err(Utf8Error { valid_up_to: i }),
                _ => {}
            }
            i += 4;
        } else {
            return Err(Utf8Error { valid_up_to: i });
        }
    }

    Ok(newlines)
}
