# simd-text

Unified SIMD text processing toolkit for Rust.

One crate for UTF-8 validation, line splitting, field extraction, number parsing, character classification, and base64 — all with consistent SIMD detection, one API style, and composable pipeline support.

## Why?

The Rust SIMD text ecosystem is fragmented: `simdutf8` for validation, `memchr` for byte search, `atoi_simd` for integers, `base64-simd` for encoding. Each has its own SIMD detection, API style, and fallback strategy. Worse, they can't fuse operations — three passes over 1 GiB costs 3x the memory bandwidth.

`simd-text` provides all these operations with:
- **One SIMD detection** — detect once, dispatch everywhere
- **One API style** — consistent error types, iterator patterns, zero-copy
- **One fallback strategy** — always correct scalar path, SIMD when available
- **Composable pipelines** — fuse multiple operations into a single pass

## Quick Start

```rust
use simd_text::{validate_utf8, line_ranges, split_fields, parse_integer};

let csv = b"name,age\nAlice,30\nBob,25\n";

// Validate + split + parse in one go
assert!(validate_utf8(csv).is_ok());

for (start, end) in line_ranges(csv).skip(1) {
    let line = &csv[start..end];
    let fields: Vec<_> = split_fields(line, b',').collect();
    let name = std::str::from_utf8(fields[0]).unwrap();
    let age: u32 = parse_integer(fields[1]).unwrap();
    println!("{name}: {age}");
}
```

## Features

| Operation | Function | SIMD? |
|-----------|----------|-------|
| UTF-8 validation | `validate_utf8()` | AVX2 |
| Line splitting | `line_ranges()` | AVX2 |
| Character classification | `CharClassifier` | AVX2 (PSHUFB) |
| Field extraction | `split_fields()` | Via classifier |
| Integer parsing | `parse_integer::<T>()` | Scalar |
| Float parsing | `parse_float::<T>()` | Scalar (Eisel-Lemire) |
| Base64 encode/decode | `base64_encode/decode()` | Scalar |
| Number extraction | `extract_numbers()` | Scalar |
| Fused pipeline | `pipeline().build()` | Per-stage |

## Pipeline API

```rust
use simd_text::pipeline;

let pipe = pipeline()
    .validate_utf8()
    .split_lines()
    .classify(b",\n\"")
    .extract_numbers()
    .build();

let results = pipe.process(data);
// results.utf8_valid, results.lines, results.classifications, results.numbers
```

## SIMD Support

| Platform | Level | Width |
|----------|-------|-------|
| x86_64 + AVX2 | Full | 256-bit |
| x86_64 + SSE4.2 | Good | 128-bit |
| aarch64 + NEON | Planned | 128-bit |
| wasm32 + SIMD | Planned | 128-bit |
| Other | Scalar | N/A |

## no_std Support

Disable the default `std` feature for `no_std` environments:

```toml
[dependencies]
simd-text = { version = "0.1", default-features = false }
```

## License

MIT OR Apache-2.0
