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
//! Git-compatible operations such as serialization, deserialization, and format
//! identification.

use crate::core::objects::traits;

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

impl traits::Serialize for Blob {
    /// Serializes the blob's data.
    ///
    /// # Returns
    /// A `Vec<u8>` containing a copy of the blob's data.
    fn serialize(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl traits::Deserialize for Blob {
    /// Deserializes a byte slice into a Blob.
    ///
    /// # Arguments
    /// * `data` - A byte slice containing the data to be deserialized.
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
    /// * `data` - A byte slice containing the data for the new Blob.
    ///
    /// # Returns
    /// A new `Blob` instance containing the provided data.
    fn from(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
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
}
