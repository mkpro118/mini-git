//! This module contains trait definitions for Git object types.
//!
//! These traits define common operations for formatting, serialization,
//! deserialization, and key-value list with message (KVLM) that are essential
//! for working with Git objects.
//!
//! The traits in this module provide a unified interface for different Git
//! object types, allowing for consistent handling of various Git internals.
//! By implementing these traits, Git object types can ensure they meet the
//! necessary requirements for Git operations and maintain compatibility with
//! the Git format.

use crate::utils::collections::kvlm;

/// Trait for Git object types that have a specific format representation.
pub trait Format {
    /// Returns the format representation of the Git object as a byte string
    fn format() -> &'static [u8];
}

/// Trait for Git object types that can be represented as a
/// Key-Value List with Messages (KVLM) structure.
///
/// This is particularly relevant for Git objects like commits and tags that
/// contain metadata in a key-value format.
pub trait KVLM {
    /// Creates a new instance of the Git object from a KVLM structure.
    ///
    /// # Arguments
    /// - `kvlm` - A KVLM structure to create the Git object instance from.
    ///   Note that the Git object will need to own that kvlm
    ///
    /// # Returns
    /// A new instance of the Git object.
    fn with_kvlm(kvlm: kvlm::KVLM) -> Self;

    /// Returns a reference to the internal KVLM structure of the Git object.
    ///
    /// # Returns
    /// A reference to the internal `kvlm::KVLM` structure.
    fn kvlm(&self) -> &kvlm::KVLM;

    /// Serializes the internal KVLM structure of the Git object.
    ///
    /// This function has a default implementation, and should only be
    /// implemented if serialization of something other than the kvlm is needed.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized KVLM data of the Git object.
    fn serialize(&self) -> Vec<u8> {
        self.kvlm().serialize()
    }

    /// Deserializes a byte slice into an instance of the Git object.
    ///
    /// This function has a default implementation, and should only be
    /// implemented if deserialization of something other than a kvlm is needed.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the Git object data to be deserialized.
    ///
    /// # Returns
    /// A `Result` containing either the deserialized Git object instance or an
    /// error message.
    ///
    /// # Errors
    /// Returns an `Err` with a `String` describing the error if parsing the
    /// KVLM structure fails.
    fn deserialize(data: &[u8]) -> Result<Self, String>
    where
        Self: Sized,
    {
        Ok(Self::with_kvlm(kvlm::KVLM::parse(data)?))
    }
}

/// Trait for Git object types that can be serialized into a byte vector.
pub trait Serialize {
    /// Serializes the Git object into a byte vector.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized Git object data.
    fn serialize(&self) -> Vec<u8>;
}

/// Trait for Git object types that can be deserialized from a byte slice.
pub trait Deserialize {
    /// Deserializes a Git object from a byte slice.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the Git object data to be deserialized.
    ///
    /// # Returns
    /// A [`Result`] containing either the deserialized Git object instance or an
    /// error message.
    ///
    /// # Errors
    /// Returns a [`String`] describing the error if deserialization fails.
    fn deserialize(data: &[u8]) -> Result<Self, String>
    where
        Self: Sized;
}
