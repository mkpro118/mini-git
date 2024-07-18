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
}
