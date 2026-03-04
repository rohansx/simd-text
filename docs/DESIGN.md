# Design Decisions

## Why one crate instead of many?

The SIMD text ecosystem is fragmented. Each crate (simdutf8, atoi_simd, base64-simd) independently detects CPU features, has its own API conventions, and can't share work. Three separate passes over the same data means 3x memory bandwidth consumed.

A unified crate enables:
1. Single `detect()` call for all operations
2. Consistent error types and iterator patterns
3. Future fused pipeline execution (one pass, multiple operations)

## Zero-copy philosophy

All iterators (`LineRanges`, `Fields`, `Records`) return byte-offset ranges or slices into the original data. No allocations except for collecting results into `Vec`.

## Scalar-first development

Every operation has a correct scalar implementation first. SIMD paths are added on top with the invariant: `simd_result == scalar_result` for all inputs. This makes testing straightforward — the scalar path is the oracle.

## Why not use `std::simd`?

`std::simd` is still nightly-only (as of early 2026). We use `#[target_feature]` + `core::arch` intrinsics for stable Rust support, with the same dispatch pattern used by memchr and simdutf8.

## Float parsing strategy

Rather than implementing Eisel-Lemire from scratch, we delegate to Rust's `str::parse::<f64>()` which already uses the Eisel-Lemire fast path internally. This gives us state-of-the-art float parsing with zero maintenance burden.

## Pipeline: sequential now, fused later

The pipeline currently executes stages sequentially. The API is designed so that a future version can fuse compatible stages (e.g., UTF-8 validation + newline scan) into a single pass without changing the public interface.
