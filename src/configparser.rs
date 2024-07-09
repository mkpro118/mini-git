use core::ops::Index;
use std::{collections::HashMap, ops::IndexMut};

#[derive(Debug)]
struct ConfigSection {
    configs: HashMap<String, String>,
}

#[derive(Debug)]
struct ConfigParser {
    sections: HashMap<String, ConfigSection>,
}

impl ConfigSection {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }
}

impl Default for ConfigSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<&str> for ConfigSection {
    type Output = String;

    fn index(&self, index: &str) -> &Self::Output {
        &self.configs[index]
    }
}

impl IndexMut<&str> for ConfigSection {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        self.configs
            .get_mut(index)
            .expect("should be able to add key")
    }
}

impl ConfigParser {
    pub fn new() -> Self {
        Self {
            sections: HashMap::new(),
        }
    }
}

impl Default for ConfigParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<&str> for ConfigParser {
    type Output = ConfigSection;

    fn index(&self, index: &str) -> &Self::Output {
        &self.sections[index]
    }
}

impl IndexMut<&str> for ConfigParser {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        self.sections
            .get_mut(index)
            .expect("should be able to add key")
    }
}
