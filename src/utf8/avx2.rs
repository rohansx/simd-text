//! AVX2-accelerated UTF-8 validation.
//!
//! Uses the Keiser-Lemire algorithm: process 32 bytes at a time,
//! validate continuation bytes via SIMD comparison and shuffle.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use super::Utf8Error;

/// Validate UTF-8 using AVX2 intrinsics.
///
/// # Safety
/// Caller must ensure AVX2 is available.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn validate_utf8_avx2(data: &[u8]) -> Result<(), Utf8Error> {
    let len = data.len();
    if len == 0 {
        return Ok(());
    }

    // For short inputs, use scalar
    if len < 32 {
        return super::scalar::validate_utf8_scalar(data);
    }

    let ptr = data.as_ptr();

    // Previous input bytes (for continuation byte checking)
    let mut _prev_input = _mm256_setzero_si256();
    let mut prev_first_len = _mm256_setzero_si256();
    let mut error = _mm256_setzero_si256();

    let mut i = 0;

    // Lookup tables for the Keiser-Lemire algorithm
    // First nibble of first byte -> expected continuation length
    let first_len_tbl = _mm256_setr_epi8(
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 3,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 3,
    );

    // High nibble of first byte -> range of valid second bytes
    let first_range_tbl = _mm256_setr_epi8(
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 8,
    );

    let range_min_tbl = _mm256_setr_epi8(
        0x00u8 as i8, 0x80u8 as i8, 0x80u8 as i8, 0x80u8 as i8,
        0xA0u8 as i8, 0x80u8 as i8, 0x90u8 as i8, 0x80u8 as i8,
        0xC2u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x00u8 as i8, 0x80u8 as i8, 0x80u8 as i8, 0x80u8 as i8,
        0xA0u8 as i8, 0x80u8 as i8, 0x90u8 as i8, 0x80u8 as i8,
        0xC2u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
    );

    let range_max_tbl = _mm256_setr_epi8(
        0x7Fu8 as i8, 0xBFu8 as i8, 0xBFu8 as i8, 0xBFu8 as i8,
        0xBFu8 as i8, 0x9Fu8 as i8, 0xBFu8 as i8, 0x8Fu8 as i8,
        0xF4u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x7Fu8 as i8, 0xBFu8 as i8, 0xBFu8 as i8, 0xBFu8 as i8,
        0xBFu8 as i8, 0x9Fu8 as i8, 0xBFu8 as i8, 0x8Fu8 as i8,
        0xF4u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
        0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8, 0x00u8 as i8,
    );

    while i + 32 <= len {
        let input = _mm256_loadu_si256(ptr.add(i) as *const __m256i);

        // Check if all ASCII (high bit clear) — fast path
        let all_ascii = _mm256_movemask_epi8(input);
        if all_ascii == 0 {
            _prev_input = input;
            prev_first_len = _mm256_setzero_si256();
            i += 32;
            continue;
        }

        // High nibble of each byte
        let high_nibbles = _mm256_and_si256(
            _mm256_srli_epi16(input, 4),
            _mm256_set1_epi8(0x0F),
        );

        // First byte expected continuation length
        let first_len = _mm256_shuffle_epi8(first_len_tbl, high_nibbles);

        // Range index based on first byte high nibble
        let range = _mm256_shuffle_epi8(first_range_tbl, high_nibbles);

        // Adjust range based on previous first_len for continuation bytes
        // Shift first_len right by 1 byte (within 128-bit lanes)
        let tmp1 = _mm256_alignr_epi8(first_len, prev_first_len, 15);
        let tmp2 = _mm256_subs_epu8(tmp1, _mm256_set1_epi8(1));
        let range = _mm256_or_si256(range, _mm256_subs_epu8(tmp2, _mm256_set1_epi8(0)));

        // Shift first_len right by 2 bytes
        let tmp1 = _mm256_alignr_epi8(first_len, prev_first_len, 14);
        let tmp2 = _mm256_subs_epu8(tmp1, _mm256_set1_epi8(2));
        let range = _mm256_or_si256(range, tmp2);

        // Look up min/max range for each byte's class
        let minv = _mm256_shuffle_epi8(range_min_tbl, range);
        let maxv = _mm256_shuffle_epi8(range_max_tbl, range);

        // Check: input[i] >= min[class] && input[i] <= max[class]
        let _err1_unused = _mm256_cmpgt_epi8(minv, _mm256_xor_si256(input, _mm256_set1_epi8(-128i8)));
        let err1 = _mm256_cmpgt_epi8(
            _mm256_xor_si256(minv, _mm256_set1_epi8(-128i8)),
            _mm256_xor_si256(input, _mm256_set1_epi8(-128i8)),
        );
        let err2 = _mm256_cmpgt_epi8(
            _mm256_xor_si256(input, _mm256_set1_epi8(-128i8)),
            _mm256_xor_si256(maxv, _mm256_set1_epi8(-128i8)),
        );

        error = _mm256_or_si256(error, _mm256_or_si256(err1, err2));

        _prev_input = input;
        prev_first_len = first_len;
        i += 32;
    }

    // Check accumulated error
    if _mm256_movemask_epi8(error) != 0 {
        // There was an error somewhere — fall back to scalar to find exact position
        return super::scalar::validate_utf8_scalar(data);
    }

    // Handle tail bytes with scalar
    if i < len {
        // Need to validate remaining bytes + check continuation from last SIMD block
        super::scalar::validate_utf8_scalar(&data[i..]).map_err(|e| Utf8Error {
            valid_up_to: i + e.valid_up_to,
        })?;
    }

    Ok(())
}
