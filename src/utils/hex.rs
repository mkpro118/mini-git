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
