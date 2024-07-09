use core::ops::Index;
use std::{collections::HashMap, ops::IndexMut};

#[derive(Debug)]
pub struct ConfigSection {
    configs: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ConfigParser {
    sections: HashMap<String, ConfigSection>,
}

impl ConfigSection {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn add_config(&mut self, key: &str, value: &str) -> &mut Self {
        self[key] = value.to_string();
        self
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
        if !self.configs.contains_key(index) {
            self.configs.insert(index.to_string(), "".to_string());
        }
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

    pub fn add_section(&mut self, section: &str) -> &mut ConfigSection {
        self.sections
            .insert(section.to_string(), ConfigSection::default());
        &mut self[section]
    }

    pub fn add_config(&mut self, section: &str, key: &str, value: &str) -> &mut Self {
        self[section][key] = value.to_string();
        self
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
    fn index_mut(&mut self, section: &str) -> &mut Self::Output {
        if !self.sections.contains_key(section) {
            self.add_section(section);
        }
        self.sections
            .get_mut(section)
            .expect("should be able to add key")
    }
}
