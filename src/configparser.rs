use core::ops::Index;
use std::borrow::Borrow;
use std::fs::canonicalize;
use std::iter::FromIterator;
use std::path::Path;
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
        let mut parser = Self {
            sections: HashMap::new(),
        };

        // Global section
        parser.add_section("");
        parser
    }

    pub fn add_section(&mut self, section: &str) -> &mut ConfigSection {
        if !self.sections.contains_key(section) {
            self.sections
                .insert(section.to_string(), ConfigSection::default());
        }
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

impl From<&str> for ConfigParser {
    fn from(value: &str) -> Self {
        // If we're able to successfully get a full path, then the str
        // more likely a path than INI text
        if let Ok(path) = canonicalize(value) {
            return Self::from(Path::new(&path));
        }

        Self::from_iter(value.split("\n"))
    }
}

impl From<&Path> for ConfigParser {
    fn from(path: &Path) -> Self {
        assert!(path.exists(), "File {:?} does not exist", path);

        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(path).expect("Should be able to open the file");
        let iter = BufReader::new(file).lines().flatten();

        Self::from_iter(iter)
    }
}

impl<'a, S> FromIterator<S> for ConfigParser
where
    S: Borrow<str>,
{
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut parser = Self::new();
        let mut curr_section = &mut parser[""];
        let iter = iter.into_iter().filter_map(|x| {
            let x = x.borrow().trim();
            if x.is_empty() || x.starts_with(";") {
                None
            } else {
                Some(x.to_owned())
            }
        });

        for line in iter {
            if line.starts_with("[") && line.ends_with("]") {
                let new_section = &line[1..(line.len() - 1)];
                parser.add_section(new_section);
                curr_section = &mut parser[new_section];
                continue;
            }
            if let Some((key, value)) = line.split_once("=") {
                curr_section[&key] = value.to_string();
            }
        }

        parser
    }
}
