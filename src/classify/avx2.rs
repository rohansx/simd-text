//! AVX2 character classification using PSHUFB nibble lookup.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Classify bytes using AVX2 PSHUFB nibble lookup.
///
/// # Safety
/// Caller must ensure AVX2 is available.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn classify_avx2(
    lo_table: &[u8; 16],
    hi_table: &[u8; 16],
    data: &[u8],
) -> Vec<usize> {
    let len = data.len();
    let mut positions = Vec::new();
    let ptr = data.as_ptr();

    // Load lookup tables into both lanes of 256-bit registers
    let lo_lut = _mm256_broadcastsi128_si256(_mm_loadu_si128(lo_table.as_ptr() as *const __m128i));
    let hi_lut = _mm256_broadcastsi128_si256(_mm_loadu_si128(hi_table.as_ptr() as *const __m128i));
    let nibble_mask = _mm256_set1_epi8(0x0F);

    let mut i = 0;
    while i + 32 <= len {
        let chunk = _mm256_loadu_si256(ptr.add(i) as *const __m256i);

        // Low nibble lookup
        let lo_nibbles = _mm256_and_si256(chunk, nibble_mask);
        let lo_result = _mm256_shuffle_epi8(lo_lut, lo_nibbles);

        // High nibble lookup
        let hi_nibbles = _mm256_and_si256(_mm256_srli_epi16(chunk, 4), nibble_mask);
        let hi_result = _mm256_shuffle_epi8(hi_lut, hi_nibbles);

        // AND: byte matches if both lookups have a common bit set
        let matched = _mm256_and_si256(lo_result, hi_result);
        let zero = _mm256_setzero_si256();
        let cmp = _mm256_cmpeq_epi8(matched, zero);
        // Invert: we want non-zero positions
        let mut mask = !(_mm256_movemask_epi8(cmp) as u32);

        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            positions.push(i + bit);
            mask &= mask - 1;
        }

        i += 32;
    }

    // Tail
    while i < len {
        let b = data[i];
        let lo = (b & 0x0F) as usize;
        let hi = (b >> 4) as usize;
        if lo_table[lo] & hi_table[hi] != 0 {
            positions.push(i);
        }
        i += 1;
    }

    positions
}
