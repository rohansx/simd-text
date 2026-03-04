/// Available SIMD instruction set levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimdLevel {
    /// No SIMD — byte-by-byte fallback.
    Scalar,
    /// x86_64 SSE 4.2 — 128-bit vectors.
    Sse42,
    /// x86_64 AVX2 — 256-bit vectors.
    Avx2,
    /// ARM64 NEON — 128-bit vectors.
    Neon,
    /// WebAssembly SIMD — 128-bit vectors.
    Wasm128,
}

/// Detect the best available SIMD level at runtime.
///
/// ```
/// let level = simd_text::detect();
/// println!("SIMD level: {:?}", level);
/// ```
pub fn detect() -> SimdLevel {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return SimdLevel::Avx2;
        }
        if is_x86_feature_detected!("sse4.2") {
            return SimdLevel::Sse42;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        // NEON is mandatory on aarch64
        return SimdLevel::Neon;
    }

    #[cfg(target_arch = "wasm32")]
    {
        // wasm32 SIMD is compile-time, not runtime
        #[cfg(target_feature = "simd128")]
        return SimdLevel::Wasm128;
    }

    SimdLevel::Scalar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_is_deterministic() {
        let a = detect();
        let b = detect();
        assert_eq!(a, b);
    }
}
