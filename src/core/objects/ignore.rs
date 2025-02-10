#![allow(dead_code, unused_variables)]

use std::io::BufRead;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};


pub enum IgnoreResult {
    NotIgnored,
    Ignored {
        ignore_file: PathBuf,
        line_number: usize,
        pattern: String,
    },
}

struct GitIgnoreEntry {
    ignore_file: PathBuf,
    should_ignore: bool,
    line_number: usize,
    pattern: String,
}

impl GitIgnoreEntry {
    fn is_ignored(&self, path: &Path) -> IgnoreResult {
        todo!()
    }

    fn parse_gitignore_pattern(
        pattern: &str,
        ignore_file: &Path,
        line_number: usize,
    ) -> Option<GitIgnoreEntry> {
        let ignore_file = ignore_file.to_path_buf();
        let raw = pattern.trim();

        if raw.is_empty() {
            return None;
        }

        let first_char = raw
            .chars()
            .next()
            .expect("Should have a first char, already checked length");

        if first_char == '#' {
            return None;
        }

        let should_ignore = '!' != first_char;

        Some(GitIgnoreEntry {
            ignore_file,
            line_number,
            should_ignore,
            pattern: match first_char {
                '!' | '\\' => raw[1..].to_owned(),
                _ => raw.to_owned(),
            },
        })
    }
}

pub struct GitIgnore {
    // todo
}

impl Default for GitIgnore {
    fn default() -> Self {
        Self::new()
    }
}

impl GitIgnore {
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    fn parse_gitignore_file(
        ignore_file_path: &Path,
    ) -> Result<Vec<GitIgnoreEntry>, String> {
        let make_err = || {
            Err(format!(
                "Could not read the file {:?}",
                ignore_file_path.as_os_str()
            ))
        };
        let Ok(file) = File::open(ignore_file_path) else {
            return make_err();
        };
        let lines = io::BufReader::new(file)
            .lines()
            .collect::<Result<Vec<_>, _>>();

        let Ok(lines) = lines else {
            return make_err();
        };

        Ok(lines
            .into_iter()
            .enumerate()
            .map(|(idx, val)| (idx + 1, val))
            .filter_map(|(line_number, pattern)| {
                GitIgnoreEntry::parse_gitignore_pattern(
                    &pattern,
                    ignore_file_path,
                    line_number,
                )
            })
            .collect::<Vec<_>>())
    }

    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> IgnoreResult {
        todo!()
    }
}
