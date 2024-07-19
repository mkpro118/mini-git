//! Utility functions for hex sequences
//!
//! This module provides functionality for encoding bytes to hexadecimal strings
//! and decoding hexadecimal strings back to bytes.

use std::num::ParseIntError;

/// Represents errors that can occur during hexadecimal decoding.
#[derive(Debug)]
pub enum DecodeHexError {
    /// The input string has an odd number of characters, which is invalid for
    /// hex representation.
    OddLength,
    /// An error occurred while parsing an integer from the hex string.
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for DecodeHexError {
    /// Converts a `ParseIntError` into a `DecodeHexError`.
    ///
    /// This implementation allows for easy conversion of `ParseIntError`s that
    /// may occur during the decoding process into our custom `DecodeHexError`
    /// type.
    ///
    /// # Arguments
    /// - `e` - The `ParseIntError` to convert.
    ///
    /// # Returns
    /// A `DecodeHexError::ParseInt` variant containing the original `ParseIntError`.
    fn from(e: ParseIntError) -> Self {
        DecodeHexError::ParseInt(e)
    }
}

/// Decodes a hexadecimal string into a vector of bytes.
///
/// # Arguments
/// - `s` - A string slice containing the hexadecimal representation to decode.
///
/// # Returns
/// A `Result` containing either a `Vec<u8>` of the decoded bytes or a
/// `DecodeHexError`.
///
/// # Errors
/// - Returns a `DecodeHexError::OddLength` if the input string has an odd number
/// of characters.
/// - Returns a `DecodeHexError::ParseInt` if any character pair fails to parse
/// as a valid hex byte.
///
/// # Examples
/// ```
/// # use mini_git::utils::hex;
/// let decoded = hex::decode("48656c6c6f").unwrap();
/// assert_eq!(decoded, vec![72, 101, 108, 108, 111]);
/// ```
pub fn decode(s: &str) -> Result<Vec<u8>, DecodeHexError> {
    if s.len() & 1 != 0 {
        return Err(DecodeHexError::OddLength);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(std::convert::Into::into)
        })
        .collect()
}

/// Encodes a slice of bytes into a hexadecimal string.
///
/// # Arguments
/// - `bytes` - A slice of bytes to encode.
///
/// # Returns
/// A `String` containing the hexadecimal representation of the input bytes.
///
/// # Examples
/// ```
/// # use mini_git::utils::hex;
/// let encoded = hex::encode(&[72, 101, 108, 108, 111]);
/// assert_eq!(encoded, "48656c6c6f");
/// ```
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, &byte| {
            s.push_str(format!("{byte:02x}").as_str());
            s
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        assert_eq!(encode(&[]), "");
        assert_eq!(encode(&[0]), "00");
        assert_eq!(encode(&[1]), "01");
        assert_eq!(encode(&[15]), "0f");
        assert_eq!(encode(&[16]), "10");
        assert_eq!(encode(&[255]), "ff");
        assert_eq!(encode(&[0, 1, 2, 3]), "00010203");
        assert_eq!(encode(&[10, 20, 30, 40]), "0a141e28");
        assert_eq!(encode(&[72, 101, 108, 108, 111]), "48656c6c6f"); // "Hello"
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode("").unwrap(), vec![]);
        assert_eq!(decode("00").unwrap(), vec![0]);
        assert_eq!(decode("01").unwrap(), vec![1]);
        assert_eq!(decode("0f").unwrap(), vec![15]);
        assert_eq!(decode("10").unwrap(), vec![16]);
        assert_eq!(decode("ff").unwrap(), vec![255]);
        assert_eq!(decode("00010203").unwrap(), vec![0, 1, 2, 3]);
        assert_eq!(decode("0A141E28").unwrap(), vec![10, 20, 30, 40]);
        assert_eq!(decode("48656C6c6f").unwrap(), vec![72, 101, 108, 108, 111]);
        // "Hello"
    }

    #[test]
    fn test_decode_odd_length() {
        assert!(matches!(decode("0"), Err(DecodeHexError::OddLength)));
        assert!(matches!(decode("abc"), Err(DecodeHexError::OddLength)));
    }

    #[test]
    fn test_decode_invalid_char() {
        assert!(matches!(decode("0g"), Err(DecodeHexError::ParseInt(_))));
        assert!(matches!(decode("gg"), Err(DecodeHexError::ParseInt(_))));
    }

    #[test]
    fn test_roundtrip() {
        let original = vec![0, 1, 2, 3, 15, 16, 255, 72, 101, 108, 108, 111];
        let encoded = encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_decode_uppercase() {
        assert_eq!(decode("48656C6C6F").unwrap(), vec![72, 101, 108, 108, 111]);
    }

    #[test]
    fn test_decode_mixed_case() {
        assert_eq!(decode("48656c6C6f").unwrap(), vec![72, 101, 108, 108, 111]);
    }
}
