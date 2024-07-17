use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TempDir {
    original_dir: PathBuf,
    test_dir: PathBuf,
}

impl TempDir {
    pub fn test_dir(&self) -> &Path {
        &self.test_dir
    }

    pub fn create(dirname: &str) -> Self {
        let salt = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Should return time")
            .as_nanos();
        let original_dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();

        let dirname = format!("{dirname}{salt}");
        let test_dir = env::temp_dir().join(&dirname);
        fs::create_dir_all(&test_dir).unwrap();
        env::set_current_dir(&test_dir).expect("Should chdir");

        Self {
            original_dir,
            test_dir,
        }
    }

    pub fn revert(&self) {
        // This may not immediately delete, so we just ignore the retval
        let _ = fs::remove_dir_all(&self.test_dir);
        println!("TRYING TO REVERT TO {:?}", &self.original_dir);
        env::set_current_dir(&self.original_dir).expect("Should revert");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let listing = walkdir(&self.test_dir);
        println!("{listing:?}");
        self.revert();
    }
}

pub fn walkdir(top: &Path) -> Vec<PathBuf> {
    assert!(top.is_dir(), "Top is not a directory (top = {top:?})");
    top.read_dir()
        .expect("Should read the dir")
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.file_stem().is_some_and(|stem| {
                !stem.to_str().is_some_and(|x| x.starts_with('.'))
            })
        })
        .fold(vec![], |mut paths, entry| {
            if entry.is_file() {
                paths.push(entry);
            } else {
                paths.extend_from_slice(&walkdir(&entry));
            }
            paths
        })
}
