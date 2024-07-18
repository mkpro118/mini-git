use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub struct OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    map: HashMap<K, V>,
    list: Vec<K>,
}

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
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> OrderedMap<K, V>
where
    K: Hash + Eq + Clone,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            list: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if !self.map.contains_key(&key) {
            self.list.push(key.clone());
        }

        self.map.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn iter(&self) -> OrderedMapIter<K, V> {
        OrderedMapIter { map: &self, idx: 0 }
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
