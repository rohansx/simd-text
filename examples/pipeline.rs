use simd_text::pipeline;

fn main() {
    let log_data = b"\
timestamp=1709567890 level=INFO latency=1.5ms status=200\n\
timestamp=1709567891 level=WARN latency=42.3ms status=503\n\
timestamp=1709567892 level=INFO latency=0.8ms status=200\n";

    // Build a pipeline that validates, splits lines, finds delimiters, and extracts numbers
    let pipe = pipeline()
        .validate_utf8()
        .split_lines()
        .classify(b"= ")
        .extract_numbers()
        .build();

    let results = pipe.process(log_data);

    println!("UTF-8 valid: {}", results.utf8_valid);
    println!("Lines: {}", results.lines.len());
    println!("Delimiter positions: {}", results.classifications.len());
    println!("Numbers found: {}", results.numbers.len());

    for span in &results.numbers {
        let num_bytes = &log_data[span.offset..span.offset + span.len];
        println!(
            "  {:?} at offset {}: {}",
            span.kind,
            span.offset,
            std::str::from_utf8(num_bytes).unwrap()
        );
    }
}
