//! A simple INI-style configuration parser for Rust.
//!
//! This module provides functionality to parse and manipulate INI-style configuration files.
//! It supports reading from strings, files, and creating configurations programmatically.
//!
//! # Examples
//!
//! Creating a configuration programmatically:
//!
//! ```
//! use mini_git::utils::configparser::ConfigParser;
//!
//! let mut config = ConfigParser::new();
//! config.add_section("database")
//!     .add_config("host", "localhost")
//!     .add_config("port", "5432");
//! config.add_config("app", "name", "MyApp");
//!
//! assert_eq!(config["database"]["host"], "localhost");
//! assert_eq!(config["app"]["name"], "MyApp");
//! ```
//!
//! Parsing a configuration from a string:
//!
//! ```
//! use mini_git::utils::configparser::ConfigParser;
//!
//! let config_str = r#"
//! [server]
//! host = 127.0.0.1
//! port = 8080
//!
//! [logging]
//! level = info
//! "#;
//!
//! let config = ConfigParser::from(config_str);
//! assert_eq!(config["server"]["host"], "127.0.0.1");
//! assert_eq!(config["logging"]["level"], "info");
//! ```

#![forbid(unsafe_code)]
#![allow(clippy::missing_panics_doc)]

use core::ops::Index;
use std::borrow::Borrow;
use std::fs::canonicalize;
use std::iter::FromIterator;
use std::path::Path;
use std::{collections::HashMap, ops::IndexMut};

/// Represents a section in the configuration.
///
/// Each section contains key-value pairs of configuration items.
///
/// # Examples
///
/// ```
/// use mini_git::utils::configparser::ConfigSection;
///
/// let mut section = ConfigSection::new();
/// section.add_config("key1", "value1")
///        .add_config("key2", "value2");
///
/// assert_eq!(section["key1"], "value1");
/// ```
#[derive(Debug)]
pub struct ConfigSection {
    configs: HashMap<String, String>,
}

/// The main configuration parser.
///
/// This struct represents the entire configuration, which consists of multiple sections.
///
/// # Examples
///
/// ```
/// use mini_git::utils::configparser::ConfigParser;
///
/// let mut config = ConfigParser::new();
/// config.add_section("database")
///     .add_config("host", "localhost")
///     .add_config("port", "5432");
///
/// assert_eq!(config["database"]["host"], "localhost");
/// ```
#[derive(Debug)]
pub struct ConfigParser {
    sections: HashMap<String, ConfigSection>,
}

impl ConfigSection {
    /// Creates a new, empty `ConfigSection`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Adds a configuration item to the section.
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the configuration item.
    /// * `value` - The value of the configuration item.
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::configparser::ConfigSection;
    ///
    /// let mut section = ConfigSection::new();
    /// section.add_config("database", "postgres")
    ///        .add_config("port", "5432");
    ///
    /// assert_eq!(section["database"], "postgres");
    /// assert_eq!(section["port"], "5432");
    /// ```
    pub fn add_config(&mut self, key: &str, value: &str) -> &mut Self {
        self[key] = value.to_string();
        self
    }

    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<isize> {
        self.configs
            .get(key)
            .map(|value| value.parse().expect("Should be parsed as float"))
    }

    #[must_use]
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.configs
            .get(key)
            .map(|value| value.parse().expect("Should be parsed as float"))
    }

    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.configs.get(key) {
            Some(value) => match value.to_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(true),
                "false" | "0" | "off" | "no" => Some(false),
                _ => None,
            },
            _ => None,
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
        &self.configs[index.trim()]
    }
}

impl IndexMut<&str> for ConfigSection {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        let index = index.trim();
        if !self.configs.contains_key(index) {
            self.configs.insert(index.to_string(), String::new());
        }
        self.configs
            .get_mut(index)
            .expect("should be able to add key")
    }
}

impl ConfigParser {
    /// Creates a new `ConfigParser` with an empty global section.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::configparser::ConfigParser;
    ///
    /// let config = ConfigParser::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let mut parser = Self {
            sections: HashMap::new(),
        };

        // Global section
        parser.add_section("");
        parser
    }

    /// Adds a new section to the configuration.
    ///
    /// # Arguments
    ///
    /// * `section` - The name of the section to add.
    ///
    /// # Returns
    ///
    /// A mutable reference to the newly added or existing section.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::configparser::ConfigParser;
    ///
    /// let mut config = ConfigParser::new();
    /// config.add_section("database")
    ///     .add_config("host", "localhost");
    ///
    /// assert_eq!(config["database"]["host"], "localhost");
    /// ```
    pub fn add_section(&mut self, section: &str) -> &mut ConfigSection {
        let section = section.trim();
        if !self.sections.contains_key(section) {
            self.sections
                .insert(section.to_string(), ConfigSection::default());
        }
        &mut self[section]
    }

    /// Adds a configuration item to a specific section.
    ///
    /// # Arguments
    ///
    /// * `section` - The name of the section.
    /// * `key` - The key of the configuration item.
    /// * `value` - The value of the configuration item.
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::configparser::ConfigParser;
    ///
    /// let mut config = ConfigParser::new();
    /// config.add_config("app", "name", "MyApp")
    ///       .add_config("app", "version", "1.0.0");
    ///
    /// assert_eq!(config["app"]["name"], "MyApp");
    /// assert_eq!(config["app"]["version"], "1.0.0");
    /// ```
    pub fn add_config(
        &mut self,
        section: &str,
        key: &str,
        value: &str,
    ) -> &mut Self {
        self[section.trim()][key.trim()] = value.trim().to_string();
        self
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigSection> {
        self.sections.get(key)
    }

    #[must_use]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut ConfigSection> {
        self.sections.get_mut(key)
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
    /// Creates a `ConfigParser` from a string.
    ///
    /// If the string is a valid path, it will be treated as a file path.
    /// Otherwise, it will be parsed as INI-style configuration text.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mini_git::utils::configparser::ConfigParser;
    ///
    /// let config_str = r#"
    /// [server]
    /// host = 127.0.0.1
    /// port = 8080
    /// "#;
    ///
    /// let config = ConfigParser::from(config_str);
    /// assert_eq!(config["server"]["host"], "127.0.0.1");
    /// assert_eq!(config["server"]["port"], "8080");
    /// ```
    fn from(value: &str) -> Self {
        // If we're able to successfully get a full path, then the str
        // more likely a path than INI text
        if let Ok(path) = canonicalize(value) {
            return Self::from(Path::new(&path));
        }

        value.split('\n').collect::<Self>()
    }
}

impl From<&Path> for ConfigParser {
    /// Creates a `ConfigParser` from a file path.
    ///
    /// # Panics
    ///
    /// Panics if the file does not exist or cannot be read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use mini_git::utils::configparser::ConfigParser;
    ///
    /// let config = ConfigParser::from(Path::new("config.ini"));
    /// // Use the config...
    /// ```
    fn from(path: &Path) -> Self {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        assert!(path.exists(), "File {path:?} does not exist");

        let file = File::open(path).expect("Should be able to open the file");
        let iter = BufReader::new(file).lines().map_while(Result::ok);

        iter.collect::<Self>()
    }
}

impl<S> FromIterator<S> for ConfigParser
where
    S: Borrow<str>,
{
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut parser = Self::new();
        let mut curr_section = &mut parser[""];
        let iter = iter.into_iter().filter_map(|x| {
            let x = x.borrow().trim();
            if x.is_empty() || x.starts_with(';') {
                None
            } else {
                Some(x.to_owned())
            }
        });

        for line in iter {
            if line.starts_with('[') && line.ends_with(']') {
                let new_section = &line[1..(line.len() - 1)];
                parser.add_section(new_section);
                curr_section = &mut parser[new_section];
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                curr_section[key.trim()] = value.trim().to_string();
            }
        }

        parser
    }
}
