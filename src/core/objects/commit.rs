//! Git Commit Object Implementation
//!
//! This module provides the implementation of the Git commit object, one of the
//! four fundamental types of Git objects (alongside blob, tree, and tag).
//! Commit objects in Git represent a single point in the project's history,
//! capturing the state of the project at a specific moment.
//!
//! The `Commit` struct encapsulates the data and behavior of a Git commit,
//! using a KVLM structure to store commit metadata such as author, committer,
//! timestamp, and commit message.
//!
//! It implements several traits from the [`traits`] module to support
//! Git-compatible operations such as serialization, deserialization,
//! and format identification.

use crate::core::objects::traits;
use crate::utils::collections::kvlm::KVLM;

/// Represents a Git commit object, encapsulating commit metadata.
#[derive(Debug)]
pub struct Commit {
    /// The key-value list with message (KVLM) structure holding commit metadata.
    pub(crate) kvlm: KVLM,
}

impl Commit {
    /// Creates a new, empty Commit object.
    ///
    /// # Returns
    /// A new `Commit` instance with an empty KVLM structure.
    #[must_use]
    pub fn new() -> Self {
        Self { kvlm: KVLM::new() }
    }
}

impl traits::Format for Commit {
    /// Returns the format identifier for Git commit objects.
    ///
    /// # Returns
    /// A static byte slice containing the ASCII representation of "commit".
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"commit";
        FORMAT
    }
}

impl traits::KVLM for Commit {
    /// Creates a new Commit instance from a given KVLM structure.
    ///
    /// # Arguments
    /// * `kvlm` - A KVLM structure containing commit metadata.
    ///
    /// # Returns
    /// A new `Commit` instance initialized with the provided KVLM data.
    fn with_kvlm(kvlm: crate::utils::collections::kvlm::KVLM) -> Self {
        Self { kvlm }
    }

    /// Returns a reference to the internal KVLM structure.
    ///
    /// # Returns
    /// A reference to the KVLM structure containing the commit's metadata.
    fn kvlm(&self) -> &crate::utils::collections::kvlm::KVLM {
        &self.kvlm
    }
}

impl Default for Commit {
    /// Creates a default (empty) Commit object.
    ///
    /// # Returns
    /// A new, empty `Commit` instance.
    fn default() -> Self {
        Self::new()
    }
}
