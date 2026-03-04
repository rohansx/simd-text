//! 64-byte aligned buffer for SIMD operations.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// A buffer aligned to 64 bytes (suitable for AVX-512, AVX2, etc.).
#[allow(dead_code)]
#[repr(C, align(64))]
pub(crate) struct AlignedBuffer {
    data: Vec<u8>,
}

#[allow(dead_code)]
impl AlignedBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn extend_from_slice(&mut self, src: &[u8]) {
        self.data.extend_from_slice(src);
    }
}
