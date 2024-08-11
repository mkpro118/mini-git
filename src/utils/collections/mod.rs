//! # Utility Collections
//!
//! This module provides the following collections:
//! - [`ordered_map::OrderedMap`], which combines the properties of a `HashMap` and a `Vec` to
//!   offer a map that maintains insertion order.
//! - [`kvlm::KVLM`], A Key-Value List with Messages, a data structure used by git to
//!   store commits and tags

pub mod kvlm;
pub mod ordered_map;
