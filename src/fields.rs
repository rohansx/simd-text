//! Delimited field extraction — zero-copy splitting by delimiter.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Split a single line into fields by delimiter. Zero-copy.
///
/// NOT a full CSV parser (no quoting/escaping) -- just the fast path.
///
/// Any byte value is accepted as the delimiter, including `\0`.
/// For empty input, a single empty field is returned (consistent with
/// how `"".split(",")` works in most languages).
///
/// ```
/// use simd_text::split_fields;
///
/// let line = b"Alice,30,NYC";
/// let fields: Vec<_> = split_fields(line, b',').collect();
/// assert_eq!(fields, vec![b"Alice".as_slice(), b"30", b"NYC"]);
///
/// // Empty input returns one empty field
/// let fields: Vec<_> = split_fields(b"", b',').collect();
/// assert_eq!(fields, vec![b"".as_slice()]);
/// ```
pub fn split_fields<'a>(line: &'a [u8], delimiter: u8) -> Fields<'a> {
    Fields {
        data: line,
        delimiter,
        pos: 0,
        done: false,
    }
}

/// Iterator over byte-slice fields.
pub struct Fields<'a> {
    data: &'a [u8],
    delimiter: u8,
    pos: usize,
    done: bool,
}

impl<'a> Iterator for Fields<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let start = self.pos;
        match self.data[start..].iter().position(|&b| b == self.delimiter) {
            Some(offset) => {
                self.pos = start + offset + 1;
                Some(&self.data[start..start + offset])
            }
            None => {
                self.done = true;
                if start <= self.data.len() {
                    Some(&self.data[start..])
                } else {
                    None
                }
            }
        }
    }
}

/// Split all lines AND fields in one pass.
///
/// SIMD finds both newlines and delimiters simultaneously. Lines are
/// split on `\n` (with `\r\n` handled), then each line is split by
/// the given delimiter.
///
/// **Note:** Do not use `b'\n'` or `b'\r'` as the delimiter, since
/// those bytes are already consumed by the line splitting pass and
/// will never appear in the per-line data.
///
/// ```
/// use simd_text::split_records;
///
/// let data = b"a,b,c\n1,2,3\n";
/// let records: Vec<Vec<&[u8]>> = split_records(data, b',')
///     .map(|fields| fields.collect())
///     .collect();
/// assert_eq!(records.len(), 2);
/// assert_eq!(records[0], vec![b"a".as_slice(), b"b", b"c"]);
/// assert_eq!(records[1], vec![b"1".as_slice(), b"2", b"3"]);
/// ```
pub fn split_records<'a>(data: &'a [u8], delimiter: u8) -> Records<'a> {
    let line_ranges: Vec<_> = crate::lines::line_ranges(data).collect();
    Records {
        data,
        delimiter,
        line_ranges,
        idx: 0,
    }
}

/// Iterator over records (lines split into fields).
pub struct Records<'a> {
    data: &'a [u8],
    delimiter: u8,
    line_ranges: Vec<(usize, usize)>,
    idx: usize,
}

impl<'a> Iterator for Records<'a> {
    type Item = Fields<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.line_ranges.len() {
            return None;
        }
        let (start, end) = self.line_ranges[self.idx];
        self.idx += 1;
        let line = &self.data[start..end];
        Some(split_fields(line, self.delimiter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_split() {
        let fields: Vec<_> = split_fields(b"a,b,c", b',').collect();
        assert_eq!(fields, vec![b"a".as_slice(), b"b", b"c"]);
    }

    #[test]
    fn single_field() {
        let fields: Vec<_> = split_fields(b"hello", b',').collect();
        assert_eq!(fields, vec![b"hello".as_slice()]);
    }

    #[test]
    fn empty_fields() {
        let fields: Vec<_> = split_fields(b",,", b',').collect();
        assert_eq!(fields, vec![b"".as_slice(), b"", b""]);
    }

    #[test]
    fn empty_input() {
        let fields: Vec<_> = split_fields(b"", b',').collect();
        assert_eq!(fields, vec![b"".as_slice()]);
    }

    #[test]
    fn tab_delimiter() {
        let fields: Vec<_> = split_fields(b"a\tb\tc", b'\t').collect();
        assert_eq!(fields, vec![b"a".as_slice(), b"b", b"c"]);
    }

    #[test]
    fn records_basic() {
        let data = b"name,age\nAlice,30\nBob,25\n";
        let records: Vec<Vec<&[u8]>> = split_records(data, b',')
            .map(|f| f.collect())
            .collect();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0], vec![b"name".as_slice(), b"age"]);
        assert_eq!(records[1], vec![b"Alice".as_slice(), b"30"]);
        assert_eq!(records[2], vec![b"Bob".as_slice(), b"25"]);
    }

    #[test]
    fn records_no_trailing_newline() {
        let data = b"a,b\nc,d";
        let records: Vec<Vec<&[u8]>> = split_records(data, b',')
            .map(|f| f.collect())
            .collect();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn null_delimiter() {
        let data = b"a\x00b\x00c";
        let fields: Vec<_> = split_fields(data, b'\0').collect();
        assert_eq!(fields, vec![b"a".as_slice(), b"b", b"c"]);
    }

    #[test]
    fn delimiter_at_boundaries() {
        // Delimiter at start and end
        let fields: Vec<_> = split_fields(b",a,", b',').collect();
        assert_eq!(fields, vec![b"".as_slice(), b"a", b""]);
    }

    #[test]
    fn records_empty_input() {
        let records: Vec<Vec<&[u8]>> = split_records(b"", b',')
            .map(|f| f.collect())
            .collect();
        assert!(records.is_empty());
    }
}
