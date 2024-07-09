use std::collections::HashMap;

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
