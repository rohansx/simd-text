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
    if len % 4 != 0 {
        return Err(DecodeError {
            position: len,
        });
    }

    let mut oi = 0;
    let mut i = 0;

    while i < len {
        let a = decode_byte(input[i], i)?;
        let b = decode_byte(input[i + 1], i + 1)?;

        if input[i + 2] == b'=' {
            // Last group, 1 output byte
            output[oi] = (a << 2) | (b >> 4);
            oi += 1;
            break;
        }

        let c = decode_byte(input[i + 2], i + 2)?;

        if input[i + 3] == b'=' {
            // Last group, 2 output bytes
            output[oi] = (a << 2) | (b >> 4);
            output[oi + 1] = (b << 4) | (c >> 2);
            oi += 2;
            break;
        }

        let d = decode_byte(input[i + 3], i + 3)?;

        output[oi] = (a << 2) | (b >> 4);
        output[oi + 1] = (b << 4) | (c >> 2);
        output[oi + 2] = (c << 6) | d;
        oi += 3;
        i += 4;
    }

    Ok(oi)
}

fn decode_byte(b: u8, pos: usize) -> Result<u8, DecodeError> {
    let v = DECODE_TABLE[b as usize];
    if v == 0xFF {
        Err(DecodeError { position: pos })
    } else if v == 0xFE {
        // Padding — shouldn't reach here via decode_byte for data chars
        Err(DecodeError { position: pos })
    } else {
        Ok(v)
    }
}
