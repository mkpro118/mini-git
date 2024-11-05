//! A hashmap wrapper that maintains insertion order.
//!
//! ## Features
//!
//! - Fast key-value lookups (average O(1))
//! - Ordered iteration based on insertion sequence
//!
//! ## Example
//!
//! ```rust
//! use mini_git::utils::collections::ordered_map::OrderedMap;
//!
//! let mut map = OrderedMap::new();
//! map.insert("first", 1);
//! map.insert("second", 2);
//! map.insert("third", 3);
//!
//! assert_eq!(map.get(&"second"), Some(&2));
//!
//! for (key, value) in &map {
//!     println!("{}: {}", key, value);
//! }
//! // Output:
//! // first: 1
//! // second: 2
//! // third: 3
//! ```

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::hash::Hash;

/// A map that preserves insertion order of its keys.
///
/// `OrderedMap<K, V>` allows fast lookups via a hash map while maintaining
/// the order of key insertions for ordered iteration.
///
/// # Type Parameters
///
/// - `K`: The key type. Must implement `Hash`, `Eq`, and `Clone`.
/// - `V`: The value type.
///
/// # Examples
///
/// ```
/// use mini_git::utils::collections::ordered_map::OrderedMap;
///
/// let mut map = OrderedMap::new();
/// map.insert("a", 1);
/// map.insert("b", 2);
///
/// assert_eq!(map.get(&"b"), Some(&2));
///
/// let keys: Vec<_> = map.iter().map(|(k, _)| k).collect();
/// assert_eq!(keys, vec![&"a", &"b"]);
/// ```
#[derive(Debug)]
pub struct OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    map: HashMap<K, V>,
    list: Vec<K>,
}

/// An iterator over the entries of an `OrderedMap`.
pub struct OrderedMapIter<'a, K, V>
where
    K: Hash + Eq + Clone,
{
    map: &'a OrderedMap<K, V>,
    idx: usize,
}

impl<K, V> Default for OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Creates an empty `OrderedMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let map: OrderedMap<String, i32> = OrderedMap::default();
    /// assert!(map.iter().next().is_none());
    /// ```
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Creates a new, empty `OrderedMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let map: OrderedMap<&str, i32> = OrderedMap::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            list: Vec::new(),
        }
    }

    /// Checks whether the key exists in the map
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let mut map: OrderedMap<i32, i32> = OrderedMap::new();
    /// map.insert(1, 2);
    /// map.insert(2, 4);
    ///
    /// assert!(map.contains_key(&1));
    /// assert!(map.contains_key(&2));
    /// assert!(!map.contains_key(&3));
    /// ```
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: core::borrow::Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the key already exists, the value is updated, but the order remains unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let mut map = OrderedMap::new();
    /// map.insert("a", 1);
    /// map.insert("b", 2);
    /// map.insert("a", 3);  // Updates value, doesn't change order
    ///
    /// let keys: Vec<_> = map.iter().map(|(k, _)| *k).collect();
    /// assert_eq!(keys, vec!["a", "b"]);
    /// assert_eq!(map.get(&"a"), Some(&3));
    /// ```
    pub fn insert(&mut self, key: K, value: V) {
        if !self.map.contains_key(&key) {
            self.list.push(key.clone());
        }

        self.map.insert(key, value);
    }

    /// Retrieves a reference to the value associated with the given key.
    ///
    /// Returns `None` if the key is not present in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let mut map = OrderedMap::new();
    /// map.insert("a", 1);
    ///
    /// assert_eq!(map.get(&"a"), Some(&1));
    /// assert_eq!(map.get(&"b"), None);
    /// ```
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    /// Retrieves a mutable reference to the value associated with the given key.
    ///
    /// Returns `None` if the key is not present in the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let mut map = OrderedMap::new();
    /// map.insert("a", 1);
    ///
    /// assert_eq!(map.get(&"a"), Some(&1));
    /// *map.get_mut(&"a").unwrap() = 42;
    /// assert_eq!(map.get(&"a"), Some(&42));
    /// ```
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    /// Returns an iterator over the key-value pairs in the map, in order of insertion.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::collections::ordered_map::OrderedMap;
    ///
    /// let mut map = OrderedMap::new();
    /// map.insert("a", 1);
    /// map.insert("b", 2);
    ///
    /// let pairs: Vec<_> = map.iter().collect();
    /// assert_eq!(pairs, vec![(&"a", &1), (&"b", &2)]);
    /// ```
    #[must_use]
    pub fn iter(&self) -> OrderedMapIter<K, V> {
        OrderedMapIter { map: self, idx: 0 }
    }
}

impl<'a, K, V> Iterator for OrderedMapIter<'a, K, V>
where
    K: Hash + Eq + Clone,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.map.list.len() {
            return None;
        }

        let idx = self.idx;
        self.idx += 1;

        let key = &self.map.list[idx];
        Some((key, &self.map.map[key]))
    }
}

impl<'a, K, V> IntoIterator for &'a OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    type IntoIter = OrderedMapIter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K, V> FromIterator<(K, V)> for OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut map = Self::new();

        for (k, v) in iter {
            map.insert(k.clone(), v.clone());
        }

        map
    }
}

impl<'a, K, V> FromIterator<(&'a K, &'a V)> for OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    fn from_iter<T: IntoIterator<Item = (&'a K, &'a V)>>(iter: T) -> Self {
        let mut map = Self::new();

        for (k, v) in iter {
            map.insert(k.clone(), v.clone());
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let map: OrderedMap<&str, i32> = OrderedMap::new();
        assert!(map.iter().next().is_none());
    }

    #[test]
    fn test_default() {
        let map: OrderedMap<&str, i32> = OrderedMap::default();
        assert!(map.iter().next().is_none());
    }

    #[test]
    fn test_insert_and_get() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("b", 2);

        assert_eq!(map.get(&"a"), Some(&1));
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.get(&"c"), None);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("a", 3);

        assert_eq!(map.get(&"a"), Some(&3));

        let keys: Vec<_> = map.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn test_iteration_order() {
        let mut map = OrderedMap::new();
        map.insert("c", 3);
        map.insert("a", 1);
        map.insert("b", 2);

        let pairs: Vec<_> = map.iter().map(|(&k, &v)| (k, v)).collect();
        assert_eq!(pairs, vec![("c", 3), ("a", 1), ("b", 2)]);
    }

    #[test]
    fn test_into_iterator() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("b", 2);

        let pairs: Vec<_> = (&map).into_iter().map(|(&k, &v)| (k, v)).collect();
        assert_eq!(pairs, vec![("a", 1), ("b", 2)]);
    }

    #[test]
    fn test_large_insert() {
        let mut map = OrderedMap::new();
        for i in 0..1000 {
            map.insert(i, i * 2);
        }

        assert_eq!(map.get(&500), Some(&1000));
        assert_eq!(map.iter().count(), 1000);
    }

    #[test]
    fn test_with_string_keys() {
        let mut map = OrderedMap::new();
        map.insert("hello".to_string(), 1);
        map.insert("world".to_string(), 2);

        assert_eq!(map.get(&"hello".to_string()), Some(&1));
        assert_eq!(map.get(&"world".to_string()), Some(&2));
    }

    #[test]
    fn test_from_iterator_empty() {
        let pairs: Vec<(&str, i32)> = vec![];
        let map: OrderedMap<&str, i32> = pairs.into_iter().collect();
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn test_from_iterator_single_pair() {
        let pairs = vec![("a", 1)];
        let map: OrderedMap<&str, i32> = pairs.into_iter().collect();
        assert_eq!(map.get(&"a"), Some(&1));
        assert_eq!(map.iter().count(), 1);
    }

    #[test]
    fn test_from_iterator_multiple_pairs() {
        let pairs = vec![("a", 1), ("b", 2), ("c", 3)];
        let map: OrderedMap<&str, i32> = pairs.into_iter().collect();

        assert_eq!(map.get(&"a"), Some(&1));
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.get(&"c"), Some(&3));
        assert_eq!(map.iter().count(), 3);
    }

    #[test]
    fn test_from_iterator_duplicate_keys() {
        let pairs = vec![("a", 1), ("b", 2), ("a", 3)];
        let map: OrderedMap<&str, i32> = pairs.into_iter().collect();

        assert_eq!(map.get(&"a"), Some(&3)); // Last inserted value should be present
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.iter().count(), 2); // "a" should only appear once
    }

    #[test]
    fn test_from_iterator_order_preserved() {
        let pairs = vec![("a", 1), ("b", 2), ("c", 3)];
        let map: OrderedMap<&str, i32> = pairs.into_iter().collect();

        let keys: Vec<_> = map.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }
}
