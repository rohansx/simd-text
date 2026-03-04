//! Integer parsing from ASCII bytes.

use super::{Integer, ParseError, ParseErrorKind};

macro_rules! impl_unsigned_integer {
    ($($ty:ty),*) => {
        $(
            impl Integer for $ty {
                fn parse_from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
                    if bytes.is_empty() {
                        return Err(ParseError { kind: ParseErrorKind::Empty, position: 0 });
                    }

                    let mut i = 0;

                    // Optional '+'
                    if bytes[0] == b'+' {
                        i += 1;
                        if i >= bytes.len() {
                            return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: i });
                        }
                    }

                    let mut result: $ty = 0;
                    let mut any_digit = false;

                    while i < bytes.len() {
                        let b = bytes[i];
                        if !b.is_ascii_digit() {
                            return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: i });
                        }
                        any_digit = true;
                        let digit = (b - b'0') as $ty;
                        result = result.checked_mul(10)
                            .and_then(|r| r.checked_add(digit))
                            .ok_or(ParseError { kind: ParseErrorKind::Overflow, position: i })?;
                        i += 1;
                    }

                    if !any_digit {
                        return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: 0 });
                    }

                    Ok(result)
                }
            }
        )*
    };
}

macro_rules! impl_signed_integer {
    ($($ty:ty => $unsigned:ty),*) => {
        $(
            impl Integer for $ty {
                fn parse_from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
                    if bytes.is_empty() {
                        return Err(ParseError { kind: ParseErrorKind::Empty, position: 0 });
                    }

                    let mut i = 0;
                    let negative = if bytes[0] == b'-' {
                        i += 1;
                        true
                    } else if bytes[0] == b'+' {
                        i += 1;
                        false
                    } else {
                        false
                    };

                    if i >= bytes.len() {
                        return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: i });
                    }

                    let mut result: $ty = 0;

                    while i < bytes.len() {
                        let b = bytes[i];
                        if !b.is_ascii_digit() {
                            return Err(ParseError { kind: ParseErrorKind::InvalidDigit, position: i });
                        }
                        let digit = (b - b'0') as $ty;
                        if negative {
                            result = result.checked_mul(10)
                                .and_then(|r| r.checked_sub(digit))
                                .ok_or(ParseError { kind: ParseErrorKind::Overflow, position: i })?;
                        } else {
                            result = result.checked_mul(10)
                                .and_then(|r| r.checked_add(digit))
                                .ok_or(ParseError { kind: ParseErrorKind::Overflow, position: i })?;
                        }
                        i += 1;
                    }

                    Ok(result)
                }
            }
        )*
    };
}

impl_unsigned_integer!(u8, u16, u32, u64, u128, usize);
impl_signed_integer!(i8 => u8, i16 => u16, i32 => u32, i64 => u64, i128 => u128, isize => usize);
