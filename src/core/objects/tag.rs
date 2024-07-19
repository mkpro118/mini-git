//! Git Tag Object Implementation
//!
//! This module provides the implementation of the Git tag object, one of the four
//! main types of Git objects (alongside blob, commit, and tree).
//! Tag objects in Git are used to mark specific points in history as important,
//! typically used for release versions.
//!
//! The `Tag` struct encapsulates the data and behavior of a Git tag, using a
//! Key Value List with Message (KVLM) structure to store tag metadata such as
//! the tagger, date, and tag message.
//!
//! It implements several traits from the [`traits`] module to support
//! Git-compatible operations such as serialization, deserialization,
//! and format identification.

use crate::core::objects::traits;
use crate::utils::collections::kvlm::KVLM;

/// Represents a Git tag object, encapsulating tag metadata.
#[derive(Debug)]
pub struct Tag {
    /// The Key Value List with Message (KVLM) structure holding tag metadata.
    pub(crate) kvlm: KVLM,
}

impl Tag {
    /// Creates a new, empty Tag object.
    ///
    /// # Returns
    /// A new `Tag` instance with an empty KVLM structure.
    #[must_use]
    pub fn new() -> Self {
        Self { kvlm: KVLM::new() }
    }
}

impl traits::Format for Tag {
    /// Returns the format identifier for Git tag objects.
    ///
    /// # Returns
    /// A static byte slice containing the ASCII representation of "tag".
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tag";
        FORMAT
    }
}

impl traits::KVLM for Tag {
    /// Creates a new Tag instance from a given KVLM structure.
    ///
    /// # Arguments
    /// - `kvlm` - A KVLM structure containing tag metadata.
    ///
    /// # Returns
    /// A new `Tag` instance initialized with the provided KVLM data.
    fn with_kvlm(kvlm: crate::utils::collections::kvlm::KVLM) -> Self {
        Self { kvlm }
    }

    /// Returns a reference to the internal KVLM structure.
    ///
    /// # Returns
    /// A reference to the KVLM structure containing the tag's metadata.
    fn kvlm(&self) -> &crate::utils::collections::kvlm::KVLM {
        &self.kvlm
    }
}

impl Default for Tag {
    /// Creates a default (empty) Tag object.
    ///
    /// # Returns
    /// A new, empty `Tag` instance.
    fn default() -> Self {
        Self::new()
    }
}
