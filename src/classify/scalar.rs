//! Scalar character classification fallback.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub(crate) fn classify_scalar(lo_table: &[u8; 16], hi_table: &[u8; 16], data: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    for (i, &b) in data.iter().enumerate() {
        let lo = (b & 0x0F) as usize;
        let hi = (b >> 4) as usize;
        if lo_table[lo] & hi_table[hi] != 0 {
            positions.push(i);
        }
    }
    positions
}
