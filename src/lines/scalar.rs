//! Scalar newline finding — byte-by-byte fallback.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub(crate) fn find_newlines_scalar(data: &[u8]) -> Vec<usize> {
    let mut positions = Vec::new();
    for (i, &b) in data.iter().enumerate() {
        if b == b'\n' {
            positions.push(i);
        }
    }
    positions
}
