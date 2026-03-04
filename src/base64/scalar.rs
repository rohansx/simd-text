//! Scalar base64 encode/decode.

use super::{DecodeError, DECODE_TABLE, ENCODE_TABLE};

pub(crate) fn encode_scalar(input: &[u8], output: &mut [u8]) -> usize {
    let len = input.len();
    let mut oi = 0;

    // Process full 3-byte groups
    let chunks = len / 3;
    for i in 0..chunks {
        let b0 = input[i * 3] as u32;
        let b1 = input[i * 3 + 1] as u32;
        let b2 = input[i * 3 + 2] as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        output[oi] = ENCODE_TABLE[((triple >> 18) & 0x3F) as usize];
        output[oi + 1] = ENCODE_TABLE[((triple >> 12) & 0x3F) as usize];
        output[oi + 2] = ENCODE_TABLE[((triple >> 6) & 0x3F) as usize];
        output[oi + 3] = ENCODE_TABLE[(triple & 0x3F) as usize];
        oi += 4;
    }

    // Handle remainder
    let rem = len % 3;
    if rem == 1 {
        let b0 = input[len - 1] as u32;
        output[oi] = ENCODE_TABLE[((b0 >> 2) & 0x3F) as usize];
        output[oi + 1] = ENCODE_TABLE[((b0 << 4) & 0x3F) as usize];
        output[oi + 2] = b'=';
        output[oi + 3] = b'=';
        oi += 4;
    } else if rem == 2 {
        let b0 = input[len - 2] as u32;
        let b1 = input[len - 1] as u32;
        output[oi] = ENCODE_TABLE[((b0 >> 2) & 0x3F) as usize];
        output[oi + 1] = ENCODE_TABLE[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize];
        output[oi + 2] = ENCODE_TABLE[((b1 << 2) & 0x3F) as usize];
        output[oi + 3] = b'=';
        oi += 4;
    }

    oi
}

pub(crate) fn decode_scalar(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
    let len = input.len();
    if len == 0 {
        return Ok(0);
    }

    // Handle non-padded input by computing the "effective" padded length
    // and how many trailing characters we have.
    //
    // Valid lengths modulo 4:
    //   0 -> full groups, no remainder
    //   2 -> 1 output byte from the trailing group
    //   3 -> 2 output bytes from the trailing group
    //   1 -> invalid (a single base64 char only gives 6 bits, not enough for a byte)
    let remainder = len % 4;
    if remainder == 1 {
        return Err(DecodeError::InvalidLength { length: len });
    }

    // Calculate required output size
    let full_groups = len / 4;
    let needed = full_groups * 3 + match remainder {
        2 => 1,
        3 => 2,
        _ => 0,
    };

    if output.len() < needed {
        return Err(DecodeError::OutputTooSmall {
            needed,
            actual: output.len(),
        });
    }

    let mut oi = 0;
    let mut i = 0;

    // Process full 4-byte groups
    let full_end = full_groups * 4;
    while i < full_end {
        let a = decode_byte(input[i], i)?;
        let b = decode_byte(input[i + 1], i + 1)?;

        if input[i + 2] == b'=' {
            // Padded: last group, 1 output byte
            output[oi] = (a << 2) | (b >> 4);
            oi += 1;
            // Remaining is padding, we're done
            return Ok(oi);
        }

        let c = decode_byte(input[i + 2], i + 2)?;

        if input[i + 3] == b'=' {
            // Padded: last group, 2 output bytes
            output[oi] = (a << 2) | (b >> 4);
            output[oi + 1] = (b << 4) | (c >> 2);
            oi += 2;
            return Ok(oi);
        }

        let d = decode_byte(input[i + 3], i + 3)?;

        output[oi] = (a << 2) | (b >> 4);
        output[oi + 1] = (b << 4) | (c >> 2);
        output[oi + 2] = (c << 6) | d;
        oi += 3;
        i += 4;
    }

    // Handle unpadded trailing characters (remainder == 2 or 3)
    if remainder == 2 {
        let a = decode_byte(input[i], i)?;
        let b = decode_byte(input[i + 1], i + 1)?;
        output[oi] = (a << 2) | (b >> 4);
        oi += 1;
    } else if remainder == 3 {
        let a = decode_byte(input[i], i)?;
        let b = decode_byte(input[i + 1], i + 1)?;
        let c = decode_byte(input[i + 2], i + 2)?;
        output[oi] = (a << 2) | (b >> 4);
        output[oi + 1] = (b << 4) | (c >> 2);
        oi += 2;
    }

    Ok(oi)
}

fn decode_byte(b: u8, pos: usize) -> Result<u8, DecodeError> {
    let v = DECODE_TABLE[b as usize];
    if v >= 0xFE {
        // 0xFF = invalid byte, 0xFE = padding char '=' in a data position
        Err(DecodeError::InvalidByte { position: pos })
    } else {
        Ok(v)
    }
}
