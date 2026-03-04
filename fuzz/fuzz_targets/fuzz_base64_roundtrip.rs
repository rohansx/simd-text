#![no_main]

use libfuzzer_sys::fuzz_target;
use simd_text::{base64_decode, base64_encode};

fuzz_target!(|data: &[u8]| {
    // --- Roundtrip: encode then decode, verify we get the original back ---
    let encoded_len = ((data.len() + 2) / 3) * 4;
    let mut encoded = vec![0u8; encoded_len];
    let enc_written = base64_encode(data, &mut encoded).expect("encode should succeed with correctly sized buffer");
    encoded.truncate(enc_written);

    // Encoded output must be a multiple of 4 bytes and consist only of
    // valid base64 characters.
    assert_eq!(
        enc_written % 4,
        0,
        "encoded length {} is not a multiple of 4 for input len {}",
        enc_written,
        data.len(),
    );

    for (i, &b) in encoded.iter().enumerate() {
        assert!(
            b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=',
            "invalid base64 byte 0x{:02X} at position {} in encoded output",
            b,
            i,
        );
    }

    // Decode back
    let decoded_len = (enc_written / 4) * 3;
    let mut decoded = vec![0u8; decoded_len];
    let dec_written = base64_decode(&encoded, &mut decoded).expect(
        &format!(
            "decode failed on our own encoded output for input len {}",
            data.len(),
        ),
    );
    decoded.truncate(dec_written);

    assert_eq!(
        &decoded[..],
        data,
        "roundtrip mismatch: input len={}, encoded len={}, decoded len={}",
        data.len(),
        enc_written,
        dec_written,
    );

    // --- Decode arbitrary data: must not panic (either Ok or Err) ---
    // Use the original fuzz input as if it were base64, just to exercise
    // the decoder on arbitrary bytes without panicking.
    if !data.is_empty() {
        let out_size = ((data.len() + 3) / 4) * 3;
        let mut out = vec![0u8; out_size];
        let _ = base64_decode(data, &mut out);
        // We don't check the result — just ensure no panic or UB.
    }
});
