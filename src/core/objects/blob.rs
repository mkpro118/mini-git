//! Git Blob Object
//!
//! This module provides the implementation of the Git blob object, which is one
//! of the four main types of Git objects (the others being commit, tree, and
//! tag). Blobs in Git are used to store the contents of files in the repository.
//!
//! The `Blob` struct represents a Git blob object and provides methods for
//! creating, accessing, and manipulating blob data.
//!
//! It implements several traits from the [`traits`] module to support
//! Git-compatible operations such as serialization, deserialization,
//! and format identification.

use crate::core::objects::traits;

const BINARY_CHECK_BYTES: usize = 8000;

/// Represents a Git blob object, which is used to store file contents in Git.
#[derive(Debug)]
pub struct Blob {
    /// The raw data content of the blob.
    pub(crate) data: Vec<u8>,
}

impl Blob {
    /// Creates a new, empty Blob.
    ///
    /// # Returns
    /// A new `Blob` instance with no data.
    #[must_use]
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Returns a reference to the blob's data.
    ///
    /// # Returns
    /// A slice containing the blob's data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Guess if the provided content is binary or not.
    ///
    /// # Returns
    /// `true` if the content is binary, `false` otherwise.
    #[must_use]
    pub fn is_binary(content: &[u8]) -> bool {
        let check_len = content.len().min(BINARY_CHECK_BYTES);
        if check_len == 0 {
            return false;
        }

        // Look for common binary file signatures
        let binary_signatures: [&[u8]; 5] = [
            &[0x7F, 0x45, 0x4C, 0x46], // ELF
            &[0x89, 0x50, 0x4E, 0x47], // PNG
            &[0x50, 0x4B, 0x03, 0x04], // ZIP
            &[0xFF, 0xD8, 0xFF],       // JPEG
            &[0x1F, 0x8B],             // GZIP
        ];

        if content.len() >= 4
            && binary_signatures.iter().any(|sig| content.starts_with(sig))
        {
            return true;
        }

        // Heuristic: check for null bytes and control characters
        let mut null_count = 0;
        let mut printable_count = 0;

        for &byte in &content[..check_len] {
            if byte == 0 {
                null_count += 1;
            } else if byte.is_ascii_graphic() || byte.is_ascii_whitespace() {
                printable_count += 1;
            }
        }

        // If more than 30% are null bytes or less than 70% are printable, consider it binary
        null_count > check_len / 3 || printable_count < (check_len * 7) / 10
    }
}

impl Default for Blob {
    /// Creates a default (empty) Blob.
    ///
    /// # Returns
    /// A new, empty `Blob` instance.
    fn default() -> Self {
        Self::new()
    }
}

impl From<&[u8]> for Blob {
    /// Creates a Blob from a byte slice.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the data for the new Blob.
    ///
    /// # Returns
    /// A new `Blob` instance containing the provided data.
    fn from(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }
}

impl traits::Format for Blob {
    /// Returns the format identifier for Git blob objects.
    ///
    /// # Returns
    /// A static byte slice containing the ASCII representation of "blob".
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"blob";
        FORMAT
    }
}

impl traits::Deserialize for Blob {
    /// Deserializes a byte slice into a Blob.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the data to be deserialized.
    ///
    /// # Returns
    /// A `Result` containing either the deserialized `Blob` instance or an error message.
    ///
    /// # Errors
    /// This implementation always succeeds, so it never returns an `Err` variant.
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        Ok(Blob {
            data: Vec::from(data),
        })
    }
}

impl traits::Serialize for Blob {
    /// Serializes the blob's data.
    ///
    /// # Returns
    /// A `Vec<u8>` containing a copy of the blob's data.
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use traits::*;

    #[test]
    fn test_blob_serialize() {
        let data = &[0; 16];
        let blob = Blob {
            data: Vec::from(data),
        };
        let serialized = blob.serialize();
        assert_eq!(&serialized, data);
    }

    #[test]
    fn test_blob_deserialize() {
        let data = &[0; 16];
        match Blob::deserialize(data) {
            Ok(Blob { data: inner }) => assert_eq!(inner, data),
            _ => panic!("Deserialize did not return a blob"),
        }
    }

    #[test]
    fn test_is_binary_with_empty_content() {
        let content = b"";
        assert!(!Blob::is_binary(content));
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_is_binary_with_control_characters() {
        let content = "This text will have >30% control characters.";
        let n = 10usize; // Number of control characters, must be <= 31
        let control_chars =
            (1u8..=(n as u8)).map(|x| x as char).collect::<Vec<_>>();

        // required_len should be > content.len() / 2
        // to make the control characters take up at least 1/3 (33%) of the content
        let required_len = (content.len() / 2) + 3;
        let chars = (0..=required_len)
            .map(|i| control_chars[i % n])
            .collect::<String>();
        let content = format!("{content}{chars}",);
        assert!(Blob::is_binary(content.as_bytes()));
    }

    #[test]
    fn test_is_binary_with_text() {
        let content = b"This is a simple ASCII text.";
        assert!(!Blob::is_binary(content));
    }

    #[test]
    fn test_is_binary_with_null_bytes() {
        let content = "This text will have >30% null bytes.";

        // required_len should be > content.len() / 2
        // to make the null bytes take up at least 1/3 (33%) of the content
        let required_len = (content.len() / 2) + 1;
        let content = format!("{content}{}", "\0".repeat(required_len));
        assert!(Blob::is_binary(content.as_bytes()));
    }

    #[test]
    fn test_is_binary_with_elf_signature() {
        let content = &[0x7F, 0x45, 0x4C, 0x46, 0x00, 0x00]; // ELF signature
        assert!(Blob::is_binary(content));
    }

    #[test]
    fn test_is_binary_with_png_signature() {
        let content = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]; // PNG signature
        assert!(Blob::is_binary(content));
    }
}
