use simd_text::{detect, validate_utf8, line_ranges, split_fields, parse_integer, CharClassifier};

fn main() {
    // Detect SIMD level
    println!("SIMD level: {:?}", detect());

    // CSV-like processing
    let data = b"name,age,score\nAlice,30,95.5\nBob,25,88.0\nCharlie,35,92.3\n";

    // Validate UTF-8
    match validate_utf8(data) {
        Ok(()) => println!("Valid UTF-8"),
        Err(e) => println!("Invalid UTF-8 at byte {}", e.valid_up_to),
    }

    // Split into lines
    let lines: Vec<_> = line_ranges(data).collect();
    println!("\n{} lines:", lines.len());

    for (i, (start, end)) in lines.iter().enumerate() {
        let line = &data[*start..*end];
        let fields: Vec<_> = split_fields(line, b',').collect();

        if i == 0 {
            // Header
            print!("  Header: ");
            for (j, f) in fields.iter().enumerate() {
                if j > 0 { print!(", "); }
                print!("{}", std::str::from_utf8(f).unwrap());
            }
            println!();
        } else {
            let name = std::str::from_utf8(fields[0]).unwrap();
            let age: u32 = parse_integer(fields[1]).unwrap();
            println!("  {} is {} years old", name, age);
        }
    }

    // Character classification
    let classifier = CharClassifier::new(b",\n");
    let delimiters = classifier.find_all(data);
    println!("\nFound {} delimiter positions", delimiters.len());
}
