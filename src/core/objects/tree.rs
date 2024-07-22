//! Git Tree Object Implementation
//!
//! This module provides the implementation of the Git tree object, one of the
//! four main types of Git objects (alongside blob, commit, and tag).
//!
//! Tree objects in Git represent directories and are used to track the
//! hierarchical structure of a repository's file system.
//!
//! It implements several traits from the [`traits`] module to support
//! Git-compatible operations such as serialization, deserialization,
//! and format identification.

use crate::core::objects::traits;
use crate::utils::hex;

/// The byte representation of a space character.
const SPACE_BYTE: u8 = b' ';
/// The byte representation of the '0' character.
const ASCII_ZERO: u8 = b'0';
/// The byte representation of a null character.
const NULL_BYTE: u8 = b'\0';
/// The size of the mode field in a tree leaf.
const MODE_SIZE: usize = 6;

/// Represents a single entry (leaf) in a Git tree object.
#[cfg_attr(test, derive(Clone))]
#[derive(Debug)]
pub struct Leaf {
    /// The mode of the entry (file permissions).
    mode: [u8; MODE_SIZE],
    /// The path (name) of the entry.
    path: Vec<u8>,
    /// The SHA-1 hash of the object this entry points to.
    sha: String,
    /// The total length of this leaf entry when serialized.
    len: usize,
}

/// Represents a Git tree object, containing multiple leaf entries.
#[derive(Debug)]
pub struct Tree {
    /// The collection of leaf entries in this tree.
    leaves: Vec<Leaf>,
}

impl Leaf {
    /// Returns the `mode` of the item
    pub fn mode(&self) -> &[u8] {
        &self.mode
    }

    /// Returns the `path` of the item
    pub fn path(&self) -> &[u8] {
        &self.path
    }

    /// Returns the SHA hex digest of the item
    pub fn sha(&self) -> &str {
        &self.sha
    }

    /// Returns the length of the leaf entry when serialized.
    ///
    /// # Returns
    /// The length of the leaf in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    // This is the key for comparing two leaves.
    // Basically git treats directory names with a trailing forward slash.
    //
    // So a directory named `foo` would be treated as `foo/`, and would be
    // sorted before a file name `foo`.
    pub fn cmp_path(&self) -> Vec<u8> {
        let mut path = self.path.clone();
        if ASCII_ZERO == self.mode[0] {
            path.push(b'/');
        }
        path
    }
}

impl PartialEq for Leaf {
    fn eq(&self, other: &Self) -> bool {
        self.sha == other.sha
    }
}

impl Eq for Leaf {}

impl PartialOrd for Leaf {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp_path().cmp(&other.cmp_path()))
    }
}

impl Ord for Leaf {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl traits::Deserialize for Leaf {
    /// Deserializes a byte slice into a Leaf object.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the serialized leaf data.
    ///
    /// # Returns
    /// A `Result` containing either the deserialized `Leaf` instance or an
    /// error message.
    ///
    /// # Errors
    /// Returns a [`String`] with a descriptive error message if deserialization
    /// fails.
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let err = |x| Err(format!("invalid tree leaf: {x}"));
        let Some(space_idx) = data.iter().position(|x| *x == SPACE_BYTE) else {
            return err("mode not found");
        };

        if space_idx < 5 {
            return err("mode is too short");
        } else if space_idx > 6 {
            return err("mode is too long");
        }

        let Some(mode) = data[..space_idx].iter().rev().enumerate().try_fold(
            [ASCII_ZERO; 6],
            |mut acc, (i, byte)| {
                if !byte.is_ascii_digit() {
                    return None;
                }

                acc[MODE_SIZE - i - 1] = *byte;
                Some(acc)
            },
        ) else {
            return err("invalid mode");
        };

        let path_start_idx = space_idx + 1;

        let Some(null_idx) = data
            .iter()
            .skip(path_start_idx)
            .position(|x| *x == NULL_BYTE)
        else {
            return err("path not found");
        };

        let null_idx = null_idx + path_start_idx;

        let path = data[path_start_idx..null_idx].to_vec();
        if path.is_empty() {
            return err("empty path");
        }

        if data.len() < null_idx + 21 {
            return err("sha not found");
        }

        let sha = hex::encode(&data[(null_idx + 1)..(null_idx + 21)]);

        Ok(Self {
            mode,
            path,
            sha,
            len: null_idx + 21,
        })
    }
}

impl traits::Serialize for Leaf {
    /// Serializes the leaf's data.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized leaf.
    fn serialize(&self) -> Vec<u8> {
        [
            match self.mode[0] {
                ASCII_ZERO => self.mode[1..].to_vec(),
                _ => self.mode.to_vec(),
            },
            vec![SPACE_BYTE],
            self.path.clone(),
            vec![NULL_BYTE],
            match hex::decode(&self.sha) {
                Ok(res) => res,
                _ => unreachable!(
                    "Invariant: Leaf with invalid sha cannot be created"
                ),
            },
        ]
        .concat()
    }
}

impl Tree {
    /// Creates a new, empty Tree object.
    ///
    /// # Returns
    /// A new `Tree` instance with no leaves.
    #[must_use]
    pub fn new() -> Self {
        Self { leaves: Vec::new() }
    }

    /// Returns the leaves/items in this tree
    pub fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }
}

impl traits::Format for Tree {
    /// Returns the format identifier for Git tree objects.
    ///
    /// # Returns
    /// A static byte slice containing the ASCII representation of "tree".
    fn format() -> &'static [u8] {
        const FORMAT: &[u8] = b"tree";
        FORMAT
    }
}

impl traits::Serialize for Tree {
    /// Serializes the Tree object into a byte vector.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized tree data.
    ///
    /// # Note
    /// This method is currently unimplemented.
    fn serialize(&self) -> Vec<u8> {
        let mut leaves = self.leaves.iter().collect::<Vec<_>>();
        leaves.sort();

        leaves.iter().map(|leaf| leaf.serialize()).fold(
            vec![],
            |mut acc, ser| {
                acc.extend_from_slice(&ser);
                acc
            },
        )
    }
}

impl traits::Deserialize for Tree {
    /// Deserializes a byte slice into a Tree object.
    ///
    /// # Arguments
    /// - `data` - A byte slice containing the serialized tree data.
    ///
    /// # Returns
    /// A `Result` containing either the deserialized `Tree` instance or an
    /// error message.
    ///
    /// # Errors
    /// Returns an `Err` with a descriptive error message if deserialization of
    /// any leaf fails.
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        let mut pos = 0;
        let mut leaves = vec![];
        while pos < data.len() {
            let leaf = Leaf::deserialize(&data[pos..])?;
            pos += leaf.len();
            leaves.push(leaf);
        }

        Ok(Self { leaves })
    }
}

impl Default for Tree {
    /// Creates a default (empty) Tree object.
    ///
    /// # Returns
    /// A new, empty `Tree` instance.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use self::traits::{Deserialize, Serialize};

    use super::*;

    fn concat_leaf(leaf: &Leaf) -> Vec<u8> {
        [
            match leaf.mode[0] {
                ASCII_ZERO => leaf.mode[1..].to_vec(),
                _ => leaf.mode.to_vec(),
            },
            vec![SPACE_BYTE],
            leaf.path.clone(),
            vec![NULL_BYTE],
            hex::decode(&leaf.sha).unwrap_or_default(),
        ]
        .concat()
    }

    fn good_data() -> [Leaf; 3] {
        [
            Leaf {
                mode: *b"010644",
                path: b"test".to_vec(),
                sha: "2".repeat(40),
                len: 0,
            },
            Leaf {
                mode: *b"100644",
                path: b"test".to_vec(),
                sha: "3".repeat(40),
                len: 0,
            },
            Leaf {
                mode: *b"100644",
                path: b"test0".to_vec(),
                sha: "1".repeat(40),
                len: 0,
            },
        ]
    }

    #[test]
    fn test_leaf_deserializer_good() {
        let mut leaves = good_data();

        for test_leaf in &mut leaves {
            let data = concat_leaf(test_leaf);
            test_leaf.len = data.len();

            let leaf = Leaf::deserialize(&data).expect("Should deserialize");

            assert_eq!(leaf, *test_leaf);
        }
    }

    #[test]
    fn test_leaf_deserializer_no_space() {
        let data = [b'0'; 32];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_no_null() {
        let data = [1, 2, 3, 4, 5, SPACE_BYTE, 10, 20];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_mode_too_short() {
        let data = [1, 2, 3, 4, SPACE_BYTE, 10, 20, NULL_BYTE, 1, 2];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_mode_too_long() {
        let data = [1, 2, 3, 4, 5, 6, 7, SPACE_BYTE, 10, 20, NULL_BYTE, 1, 2];
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_hex_too_short() {
        let leaf = Leaf {
            mode: *b"100644",
            path: b"test".to_vec(),
            sha: "t".repeat(19),
            len: 0,
        };

        let data = concat_leaf(&leaf);
        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_bad_mode() {
        let leaf = Leaf {
            mode: *b"abcdef",
            path: b"test".to_vec(),
            sha: "t".repeat(20),
            len: 0,
        };

        let data = concat_leaf(&leaf);

        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_empty_path() {
        let leaf = Leaf {
            mode: *b"100644",
            path: b"".to_vec(),
            sha: "a".repeat(20),
            len: 0,
        };

        let data = concat_leaf(&leaf);

        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_leaf_deserializer_bad_hex() {
        let leaf = Leaf {
            mode: *b"100644",
            path: b"test".to_vec(),
            sha: "t".repeat(20),
            len: 0,
        };

        let data = concat_leaf(&leaf);

        let res = Leaf::deserialize(&data);
        assert!(res.is_err());
    }

    #[test]
    fn test_tree_deserialize_good() {
        let mut good_data = good_data();
        let leaves = good_data
            .iter_mut()
            .map(|leaf| {
                let res = concat_leaf(leaf);
                leaf.len = res.len();
                res
            })
            .fold(vec![], |mut acc, leaf| {
                acc.extend_from_slice(&leaf);
                acc
            });

        let tree = Tree::deserialize(&leaves).expect("Should deserialize");

        for (leaf, known_leaf) in tree.leaves.iter().zip(good_data.iter()) {
            assert_eq!(leaf, known_leaf);
        }
    }

    #[test]
    fn test_tree_deserialize_bad() {
        let mut good_data = good_data();
        let leaves = good_data
            .iter_mut()
            .map(|leaf| {
                let res = concat_leaf(leaf);
                leaf.len = res.len();
                res
            })
            .fold(vec![], |mut acc, leaf| {
                acc.extend_from_slice(&leaf);
                acc.extend_from_slice(b"extra!");
                acc
            });

        let tree = Tree::deserialize(&leaves);
        assert!(tree.is_err());
    }

    #[test]
    fn test_leaf_serialize_good_manual() {
        let leaf = Leaf {
            mode: *b"000644",
            path: b"leaf".to_vec(),
            sha: "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391".to_owned(),
            len: 0,
        };

        let expected = b"00644 leaf\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b\
        )\xaewZ\xd8\xc2\xe4\x8cS\x91";

        let serialized = leaf.serialize();

        assert_eq!(serialized, *expected);
    }

    #[test]
    fn test_leaf_serialize_good() {
        let data = good_data();

        for leaf in data {
            let test_serialize = concat_leaf(&leaf);
            let leaf_serialize = leaf.serialize();

            assert_eq!(test_serialize, leaf_serialize);
        }
    }

    #[test]
    fn test_tree_serialize_good() {
        let leaves = good_data();
        let tree = Tree {
            leaves: leaves.to_vec(),
        };

        let expected_serialized = [
            leaves[1].serialize(),
            leaves[0].serialize(),
            leaves[2].serialize(),
        ]
        .concat();

        let serialized = tree.serialize();
        assert_eq!(expected_serialized, serialized);
    }
}
