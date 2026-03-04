//! Line splitting with SIMD acceleration.
//!
//! Split input into lines by finding newline boundaries (`\n` and `\r\n`).

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

mod scalar;
#[cfg(target_arch = "x86_64")]
mod avx2;

/// Split input into lines, returning byte-offset ranges for each line.
///
/// Handles LF (`\n`), CRLF (`\r\n`), and mixed line endings. Strips
/// the line ending from each range. Trailing content without a final
/// newline is included as the last line.
///
/// ```
/// use simd_text::line_ranges;
///
/// let data = b"hello\nworld\n";
/// let lines: Vec<_> = line_ranges(data).collect();
/// assert_eq!(lines, vec![(0, 5), (6, 11)]);
///
/// // Content without trailing newline
/// let data = b"a\nb";
/// let lines: Vec<_> = line_ranges(data).collect();
/// assert_eq!(lines, vec![(0, 1), (2, 3)]);
/// ```
pub fn line_ranges(data: &[u8]) -> LineRanges<'_> {
    // Find all newline positions
    let positions = find_newlines(data);
    LineRanges {
        data,
        positions,
        idx: 0,
        prev_end: 0,
    }
}

fn find_newlines(data: &[u8]) -> Vec<usize> {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // SAFETY: AVX2 confirmed
            return unsafe { avx2::find_newlines_avx2(data) };
        }
    }
    scalar::find_newlines_scalar(data)
}

/// Iterator over line ranges `(start, end)` in a byte slice.
pub struct LineRanges<'a> {
    data: &'a [u8],
    positions: Vec<usize>,
    idx: usize,
    prev_end: usize,
}

impl<'a> Iterator for LineRanges<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.positions.len() {
            let nl_pos = self.positions[self.idx];
            self.idx += 1;

            let start = self.prev_end;
            // Strip \r if CRLF
            let end = if nl_pos > 0 && self.data[nl_pos - 1] == b'\r' {
                nl_pos - 1
            } else {
                nl_pos
            };
            self.prev_end = nl_pos + 1;
            Some((start, end))
        } else if self.prev_end < self.data.len() {
            // Trailing content without final newline
            let start = self.prev_end;
            let end = self.data.len();
            self.prev_end = end;
            Some((start, end))
        } else {
            None
        }
    }
}

/// Streaming line scanner for readers.
///
/// Reads from an `impl Read` source and yields lines one at a time
/// without loading the entire input into memory.
#[cfg(feature = "std")]
pub struct LineScanner<R: std::io::Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    filled: usize,
    done: bool,
}

#[cfg(feature = "std")]
impl<R: std::io::Read> LineScanner<R> {
    /// Create a new streaming line scanner.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: vec![0u8; 64 * 1024], // 64 KiB buffer
            pos: 0,
            filled: 0,
            done: false,
        }
    }

    /// Read the next line. Returns `None` at EOF.
    pub fn next_line(&mut self) -> Option<&[u8]> {
        loop {
            // Search for newline in buffered data
            if let Some(nl) = self.buf[self.pos..self.filled]
                .iter()
                .position(|&b| b == b'\n')
            {
                let start = self.pos;
                let end = self.pos + nl;
                self.pos = end + 1;
                // Strip \r if CRLF
                let end = if end > start && self.buf[end - 1] == b'\r' {
                    end - 1
                } else {
                    end
                };
                return Some(&self.buf[start..end]);
            }

            if self.done {
                if self.pos < self.filled {
                    let start = self.pos;
                    self.pos = self.filled;
                    return Some(&self.buf[start..self.filled]);
                }
                return None;
            }

            // Compact buffer
            if self.pos > 0 {
                self.buf.copy_within(self.pos..self.filled, 0);
                self.filled -= self.pos;
                self.pos = 0;
            }

            // Grow buffer if full
            if self.filled == self.buf.len() {
                self.buf.resize(self.buf.len() * 2, 0);
            }

            // Read more data
            match self.reader.read(&mut self.buf[self.filled..]) {
                Ok(0) => self.done = true,
                Ok(n) => self.filled += n,
                Err(_) => self.done = true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let lines: Vec<_> = line_ranges(b"").collect();
        assert!(lines.is_empty());
    }

    #[test]
    fn single_line_no_newline() {
        let lines: Vec<_> = line_ranges(b"hello").collect();
        assert_eq!(lines, vec![(0, 5)]);
    }

    #[test]
    fn single_line_with_newline() {
        let lines: Vec<_> = line_ranges(b"hello\n").collect();
        assert_eq!(lines, vec![(0, 5)]);
    }

    #[test]
    fn multiple_lines() {
        let lines: Vec<_> = line_ranges(b"a\nb\nc\n").collect();
        assert_eq!(lines, vec![(0, 1), (2, 3), (4, 5)]);
    }

    #[test]
    fn crlf() {
        let lines: Vec<_> = line_ranges(b"hello\r\nworld\r\n").collect();
        assert_eq!(lines, vec![(0, 5), (7, 12)]);
    }

    #[test]
    fn mixed_endings() {
        let lines: Vec<_> = line_ranges(b"a\nb\r\nc\n").collect();
        assert_eq!(lines, vec![(0, 1), (2, 3), (5, 6)]);
    }

    #[test]
    fn empty_lines() {
        let lines: Vec<_> = line_ranges(b"\n\n\n").collect();
        assert_eq!(lines, vec![(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn trailing_without_newline() {
        let lines: Vec<_> = line_ranges(b"a\nb").collect();
        assert_eq!(lines, vec![(0, 1), (2, 3)]);
    }

    #[test]
    fn large_input() {
        let mut data = Vec::new();
        for _ in 0..1000 {
            data.extend_from_slice(b"some line content here\n");
        }
        let lines: Vec<_> = line_ranges(&data).collect();
        assert_eq!(lines.len(), 1000);
    }

    #[cfg(feature = "std")]
    #[test]
    fn line_scanner_basic() {
        let data = b"hello\nworld\n";
        let mut scanner = LineScanner::new(&data[..]);
        assert_eq!(scanner.next_line(), Some(b"hello".as_slice()));
        assert_eq!(scanner.next_line(), Some(b"world".as_slice()));
        assert_eq!(scanner.next_line(), None);
    }
}
