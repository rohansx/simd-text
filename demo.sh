#!/bin/bash
# simd-text demo — Unified SIMD text processing
set -e

echo "═══════════════════════════════════════════════════"
echo "  simd-text — Unified SIMD Text Processing Toolkit"
echo "═══════════════════════════════════════════════════"
echo ""
sleep 1

echo ">> One crate for: UTF-8 validation, line splitting, character"
echo "   classification, field extraction, number parsing, and base64"
echo "   All with AVX2 SIMD acceleration."
echo ""
sleep 2

echo ">> Running tests (90 unit + 15 doc)..."
cargo test --quiet 2>&1
echo ""
sleep 1

echo ">> CSV-like processing example..."
echo "   (Validate UTF-8, split lines, extract fields, parse numbers)"
echo ""
cargo run --example basic --quiet 2>&1
echo ""
sleep 2

echo ">> Pipeline example..."
echo "   (Fused multi-stage processing on log data)"
echo ""
cargo run --example pipeline --quiet 2>&1
echo ""
sleep 2

echo ">> Base64 roundtrip example..."
echo ""
cargo run --example base64_demo --quiet 2>&1
echo ""
sleep 2

echo ">> Benchmark highlights (AVX2):"
echo "   UTF-8 validation:     45.6 GiB/s"
echo "   Char classification:  16.3 GiB/s"
echo "   Line splitting:        5.2 GiB/s"
echo "   Base64 encode:         1.2 GiB/s (scalar — SIMD coming)"
echo "   Integer parsing:       ~9 ns/number"
echo ""
sleep 1

echo ">> Running live benchmark (UTF-8 validation, 1 MiB)..."
cargo bench -- "utf8_validate/1048576" --quiet 2>&1 | grep -E "(time:|thrpt:)" | head -2
echo ""
sleep 1

echo ">> Key features:"
echo "   - Consistent SIMD detection (detect once, dispatch everywhere)"
echo "   - Zero-copy iterators (LineRanges, Fields, Records)"
echo "   - Composable pipeline API for fused single-pass processing"
echo "   - no_std support"
echo "   - 5 fuzz targets for safety"
echo ""
echo ">> cargo add simd-text  # coming soon to crates.io"
echo ""
echo "═══════════════════════════════════════════════════"
