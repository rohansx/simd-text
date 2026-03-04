#![no_main]

use libfuzzer_sys::fuzz_target;
use simd_text::validate_utf8;

fuzz_target!(|data: &[u8]| {
    let ours = validate_utf8(data);
    let std_result = std::str::from_utf8(data);

    match (&ours, &std_result) {
        (Ok(()), Ok(_)) => {
            // Both agree the input is valid UTF-8 — good.
        }
        (Err(our_err), Err(std_err)) => {
            // Both agree the input is invalid.
            // The first invalid byte position must match.
            assert_eq!(
                our_err.valid_up_to,
                std_err.valid_up_to(),
                "valid_up_to mismatch: ours={} std={} on input of len {}",
                our_err.valid_up_to,
                std_err.valid_up_to(),
                data.len(),
            );
        }
        (Ok(()), Err(std_err)) => {
            panic!(
                "we said Ok but std said Err at byte {} on input of len {}",
                std_err.valid_up_to(),
                data.len(),
            );
        }
        (Err(our_err), Ok(_)) => {
            panic!(
                "we said Err at byte {} but std said Ok on input of len {}",
                our_err.valid_up_to,
                data.len(),
            );
        }
    }
});
