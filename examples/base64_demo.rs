use simd_text::{base64_encode, base64_decode};

fn main() {
    let messages = [
        "Hello, World!",
        "SIMD text processing is fast!",
        "Base64 encoding: ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    ];

    for msg in &messages {
        let input = msg.as_bytes();
        let mut encoded = vec![0u8; input.len() * 2];
        let enc_len = base64_encode(input, &mut encoded).unwrap();
        let encoded = &encoded[..enc_len];

        let mut decoded = vec![0u8; encoded.len()];
        let dec_len = base64_decode(encoded, &mut decoded).unwrap();
        let decoded = &decoded[..dec_len];

        println!("Original:  {}", msg);
        println!("Encoded:   {}", std::str::from_utf8(encoded).unwrap());
        println!("Decoded:   {}", std::str::from_utf8(decoded).unwrap());
        println!("Roundtrip: {}", input == decoded);
        println!();
    }
}
