use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};

fn gen_ascii(size: usize) -> Vec<u8> {
    (0..size).map(|i| b'A' + (i % 26) as u8).collect()
}

fn gen_lines(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let mut col = 0;
    for i in 0..size {
        if col >= 80 {
            data.push(b'\n');
            col = 0;
        } else {
            data.push(b'A' + (i % 26) as u8);
            col += 1;
        }
    }
    data
}

fn bench_utf8_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("utf8_validate");

    for size in [1024, 64 * 1024, 1024 * 1024] {
        let data = gen_ascii(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| simd_text::validate_utf8(data))
        });
    }

    group.finish();
}

fn bench_line_ranges(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_ranges");

    for size in [1024, 64 * 1024, 1024 * 1024] {
        let data = gen_lines(size);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| {
                let lines: Vec<_> = simd_text::line_ranges(data).collect();
                lines
            })
        });
    }

    group.finish();
}

fn bench_classify(c: &mut Criterion) {
    let mut group = c.benchmark_group("classify");
    let classifier = simd_text::CharClassifier::new(b" \t\n,");

    for size in [1024, 64 * 1024, 1024 * 1024] {
        let data = gen_lines(size);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| classifier.find_all(data))
        });
    }

    group.finish();
}

fn bench_base64(c: &mut Criterion) {
    let mut group = c.benchmark_group("base64_encode");

    for size in [1024, 64 * 1024, 256 * 1024] {
        let data = gen_ascii(size);
        let mut output = vec![0u8; size * 2];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| simd_text::base64_encode(data, &mut output))
        });
    }

    group.finish();
}

fn bench_parse_integer(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_integer");

    let numbers: Vec<&[u8]> = vec![
        b"0", b"42", b"12345", b"999999999", b"-2147483648",
    ];

    group.bench_function("mixed", |b| {
        b.iter(|| {
            for num in &numbers {
                let _: i64 = simd_text::parse_integer(num).unwrap();
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_utf8_validation,
    bench_line_ranges,
    bench_classify,
    bench_base64,
    bench_parse_integer,
);
criterion_main!(benches);
