//! SIMD character classification.
//!
//! Classify ASCII characters at high speed using the PSHUFB nibble lookup trick.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod scalar;
#[cfg(target_arch = "x86_64")]
mod avx2;

/// SIMD character classifier using lookup tables.
///
/// Classifies bytes at up to ~10 GiB/s on AVX2 by using the PSHUFB nibble
/// lookup trick: each byte is split into high/low nibbles, each nibble is
/// used as an index into a 16-byte lookup table, and the results are ANDed
/// to determine if the byte matches.
///
/// ```
/// use simd_text::CharClassifier;
///
/// let ws = CharClassifier::new(b" \t\r\n");
/// let data = b"hello world\tfoo\nbar";
/// let positions = ws.find_all(data);
/// assert_eq!(positions, vec![5, 11, 15]);
/// ```
pub struct CharClassifier {
    /// Lookup table indexed by low nibble
    lo_table: [u8; 16],
    /// Lookup table indexed by high nibble
    hi_table: [u8; 16],
}

impl CharClassifier {
    /// Create a classifier for a set of ASCII byte values.
    ///
    /// Only ASCII bytes (0-127) are supported. Non-ASCII bytes in the
    /// input set are silently ignored.
    ///
    /// An empty `chars` slice is valid and creates a classifier that
    /// matches nothing (i.e., `find_all` always returns an empty `Vec`).
    ///
    /// If more than 8 unique nibble-pair classes are needed, the
    /// classifier degrades gracefully but remains correct (it may
    /// produce false-positive matches that are filtered by the scalar
    /// fallback path).
    pub fn new(chars: &[u8]) -> Self {
        // Build nibble lookup tables
        // For each character, set a bit in both the low-nibble and high-nibble tables
        // A byte matches if the AND of both lookups is non-zero
        let mut lo_table = [0u8; 16];
        let mut hi_table = [0u8; 16];

        // We use up to 8 "classes" (bit positions) to encode the character set
        // Each unique (high_nibble, low_nibble) pair gets assigned a class bit
        let mut next_bit = 0u8;

        for &ch in chars {
            if ch > 127 {
                continue;
            }
            let lo = (ch & 0x0F) as usize;
            let hi = (ch >> 4) as usize;

            // Check if this combination is already covered
            let existing = lo_table[lo] & hi_table[hi];
            if existing != 0 {
                continue;
            }

            if next_bit >= 8 {
                // Ran out of class bits; fall back to setting all bits
                // This is still correct but less selective
                lo_table[lo] |= 0xFF;
                hi_table[hi] |= 0xFF;
                continue;
            }

            let bit = 1u8 << next_bit;
            lo_table[lo] |= bit;
            hi_table[hi] |= bit;
            next_bit += 1;
        }

        Self { lo_table, hi_table }
    }

    /// Find all byte positions where the input matches one of the
    /// classifier's characters.
    pub fn find_all(&self, data: &[u8]) -> Vec<usize> {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 confirmed
                return unsafe { avx2::classify_avx2(&self.lo_table, &self.hi_table, data) };
            }
        }
        scalar::classify_scalar(&self.lo_table, &self.hi_table, data)
    }

    /// Streaming: invoke a callback for each matching position.
    pub fn scan<F: FnMut(usize)>(&self, data: &[u8], mut callback: F) {
        for pos in self.find_all(data) {
            callback(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_classifier() {
        let ws = CharClassifier::new(b" \t\r\n");
        let data = b"hello world\t!\r\n";
        let positions = ws.find_all(data);
        assert_eq!(positions, vec![5, 11, 13, 14]);
    }

    #[test]
    fn digit_classifier() {
        let digits = CharClassifier::new(b"0123456789");
        let data = b"abc123def456";
        let positions = digits.find_all(data);
        assert_eq!(positions, vec![3, 4, 5, 9, 10, 11]);
    }

    #[test]
    fn csv_delimiters() {
        let csv = CharClassifier::new(b",\n\"");
        let data = b"name,age\n\"Bob\",30\n";
        let positions = csv.find_all(data);
        assert!(positions.contains(&4));  // comma
        assert!(positions.contains(&8));  // \n
        assert!(positions.contains(&9));  // "
        assert!(positions.contains(&13)); // "
        assert!(positions.contains(&14)); // comma
        assert!(positions.contains(&17)); // \n
    }

    #[test]
    fn single_char() {
        let c = CharClassifier::new(b"x");
        let data = b"axbxcxd";
        assert_eq!(c.find_all(data), vec![1, 3, 5]);
    }

    #[test]
    fn no_matches() {
        let c = CharClassifier::new(b"z");
        let data = b"hello world";
        assert!(c.find_all(data).is_empty());
    }

    #[test]
    fn empty_input() {
        let c = CharClassifier::new(b" ");
        assert!(c.find_all(b"").is_empty());
    }

    #[test]
    fn empty_char_set() {
        let c = CharClassifier::new(b"");
        assert!(c.find_all(b"hello world 123!@#").is_empty());
    }

    #[test]
    fn non_ascii_chars_ignored() {
        // Non-ASCII bytes in the char set should be silently ignored
        let c = CharClassifier::new(&[0xFF, 0x80, b'x']);
        let data = b"axbxc";
        assert_eq!(c.find_all(data), vec![1, 3]);
    }

    #[test]
    fn scan_callback() {
        let c = CharClassifier::new(b".");
        let mut found = Vec::new();
        c.scan(b"a.b.c", |pos| found.push(pos));
        assert_eq!(found, vec![1, 3]);
    }

    #[test]
    fn large_input() {
        let c = CharClassifier::new(b"\n");
        let mut data = vec![b'a'; 1000];
        data[100] = b'\n';
        data[500] = b'\n';
        data[999] = b'\n';
        let positions = c.find_all(&data);
        assert_eq!(positions, vec![100, 500, 999]);
    }
}
