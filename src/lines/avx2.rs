//! AVX2-accelerated newline finding.
//!
//! Process 32 bytes at a time, comparing against `\n`.

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Find all newline (`\n`) positions using AVX2.
///
/// # Safety
/// Caller must ensure AVX2 is available.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn find_newlines_avx2(data: &[u8]) -> Vec<usize> {
    let len = data.len();
    let mut positions = Vec::new();
    let ptr = data.as_ptr();
    let newline = _mm256_set1_epi8(b'\n' as i8);

    let mut i = 0;
    while i + 32 <= len {
        let chunk = _mm256_loadu_si256(ptr.add(i) as *const __m256i);
        let cmp = _mm256_cmpeq_epi8(chunk, newline);
        let mut mask = _mm256_movemask_epi8(cmp) as u32;

        while mask != 0 {
            let bit = mask.trailing_zeros() as usize;
            positions.push(i + bit);
            mask &= mask - 1;
        }

        i += 32;
    }

    // Handle tail
    while i < len {
        if data[i] == b'\n' {
            positions.push(i);
        }
        i += 1;
    }

    positions
}
