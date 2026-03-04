# Architecture

## Module Structure

```
src/
├── lib.rs              Public API, re-exports
├── detect_mod.rs       SimdLevel enum + runtime detection
├── aligned.rs          64-byte aligned buffer for SIMD ops
│
├── utf8/
│   ├── mod.rs          validate_utf8, validate_and_split_lines
│   ├── avx2.rs         AVX2 Keiser-Lemire validation
│   └── scalar.rs       Byte-by-byte scalar fallback
│
├── lines/
│   ├── mod.rs          line_ranges, LineScanner
│   ├── avx2.rs         32-byte newline scan
│   └── scalar.rs       Scalar fallback
│
├── classify/
│   ├── mod.rs          CharClassifier (PSHUFB trick)
│   ├── avx2.rs         AVX2 nibble lookup
│   └── scalar.rs       Scalar lookup table
│
├── fields.rs           split_fields, split_records
│
├── numbers/
│   ├── mod.rs          parse_integer, parse_float, extract_numbers
│   ├── integer.rs      Integer parsing with overflow checks
│   └── float.rs        Float parsing (delegates to std Eisel-Lemire)
│
├── base64/
│   ├── mod.rs          base64_encode, base64_decode
│   └── scalar.rs       Standard base64 scalar impl
│
└── pipeline_mod.rs     PipelineBuilder, Pipeline, fused execution
```

## SIMD Dispatch Pattern

Every SIMD-accelerated module follows the same pattern:

1. Public function in `mod.rs` checks CPU features at runtime
2. Dispatches to `avx2.rs` (or `sse42.rs`, `neon.rs`) if available
3. Falls back to `scalar.rs` otherwise

```rust
pub fn operation(data: &[u8]) -> Result {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            return unsafe { avx2::operation_avx2(data) };
        }
    }
    scalar::operation_scalar(data)
}
```

All `unsafe` SIMD code is:
- Inside `#[target_feature(enable = "...")]` functions
- Called only after runtime feature detection
- Paired with a scalar fallback producing identical results

## Key SIMD Techniques

### PSHUFB Nibble Lookup (Character Classification)

The `_mm256_shuffle_epi8` instruction treats a 16-byte register as a lookup table indexed by the low 4 bits of each byte. By splitting each input byte into high/low nibbles and doing two lookups, we can classify 32 bytes in ~3 cycles.

### AVX2 Newline Scan

Compare 32 bytes against `\n` using `_mm256_cmpeq_epi8`, extract bitmask with `_mm256_movemask_epi8`, enumerate set bits with `trailing_zeros`.

### Keiser-Lemire UTF-8 Validation

Process 32 bytes per iteration using lookup tables that encode valid UTF-8 byte ranges. Fast-path: if no high bits set (all ASCII), skip validation entirely.
