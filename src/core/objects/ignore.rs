use std::path::{Path, PathBuf};

use crate::utils::path::{current_dir, home_dir, to_posix_path};

pub enum IgnoreResult {
    NotIgnored,
    Ignored {
        ignore_file: PathBuf,
        line_number: usize,
        pattern: String,
    },
}

struct GitIgnoreEntry {
    // todo
}

impl GitIgnoreEntry {
    fn is_ignored(&self, path: &Path) -> IgnoreResult {
        todo!()
    }
}

pub struct GitIgnore {
    // todo
}

impl GitIgnore {
    pub fn new() -> Self {
        todo!()
    }

    pub fn is_ignored(&self, path: &Path) -> IgnoreResult {
        todo!()
    }
}
