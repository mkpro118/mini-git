//! # Test Utilities Module
//!
//! This module provides utilities for setting up and managing temporary directories
//! for testing purposes. It includes functionality for creating isolated test
//! environments, reverting changes, and walking directory structures.
//!
//! ## Main Components
//!
//! - [`TempDir`]: A struct for creating and managing temporary directories.
//! - [`walkdir`]: A function for recursively listing files in a directory.
//!
//! ## Usage
//!
//! This module is primarily intended for use in test scenarios where isolated
//! file system operations are required. It allows for easy creation of temporary
//! directories that are automatically cleaned up when they go out of scope.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// A struct representing a temporary directory for testing purposes.
///
/// This struct manages the creation and cleanup of a temporary directory,
/// as well as changing the current working directory to the temporary directory
/// and reverting it back when done.
///
/// # Examples
///
/// ```
/// use mini_git::utils::test::TempDir;
///
/// let temp_dir = TempDir::create("my_test");
/// // Perform test operations in the temporary directory
/// // The directory will be automatically cleaned up when `temp_dir` goes out of scope
/// ```
pub struct TempDir {
    original_dir: PathBuf,
    test_dir: PathBuf,
}

impl TempDir {
    /// Returns a reference to the path of the temporary test directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::create("my_test");
    /// let test_path = temp_dir.test_dir();
    /// println!("Temporary directory path: {:?}", test_path);
    /// ```
    #[must_use]
    pub fn test_dir(&self) -> &Path {
        &self.test_dir
    }

    /// Creates a new temporary directory for testing.
    ///
    /// This function creates a unique temporary directory based on the provided
    /// `dirname` and a timestamp. It also changes the current working directory
    /// to the newly created temporary directory.
    ///
    /// # Arguments
    ///
    /// * `dirname` - A base name for the temporary directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::create("my_test");
    /// // The current working directory is now the temporary directory
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - It fails to get the current system time.
    /// - It fails to create the temporary directory.
    /// - It fails to change the current working directory to the temporary directory.
    #[must_use]
    pub fn create(dirname: &str) -> Self {
        let salt = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Should return time")
            .as_nanos();
        let original_dir = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();

        let dirname = format!("{dirname}{salt}");
        let test_dir = env::temp_dir().join(dirname);
        fs::create_dir_all(&test_dir).unwrap();
        env::set_current_dir(&test_dir).expect("Should chdir");

        Self {
            original_dir,
            test_dir,
        }
    }

    /// Reverts the current working directory to the original directory
    /// and attempts to remove the temporary directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use mini_git::utils::test::TempDir;
    ///
    /// let temp_dir = TempDir::create("my_test");
    /// // Perform test operations
    /// temp_dir.revert();
    /// // The current working directory is now back to the original
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if it fails to change the current working
    /// directory back to the original directory.
    pub fn revert(&self) {
        // This may not immediately delete, so we just ignore the retval
        let _ = fs::remove_dir_all(&self.test_dir);
        env::set_current_dir(&self.original_dir).expect("Should revert");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        self.revert();
    }
}

/// Recursively walks a directory and returns a vector of all file paths.
///
/// This function traverses the given directory and its subdirectories,
/// collecting the paths of all files (excluding hidden files) into a vector.
///
/// # Arguments
///
/// * `top` - The path to the directory to walk.
///
/// # Returns
///
/// A `Vec<PathBuf>` containing the paths of all non-hidden files in the directory
/// and its subdirectories.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use mini_git::utils::test::walkdir;
///
/// let dir_path = Path::new("/path/to/directory");
/// let files = walkdir(dir_path);
/// for file in files {
///     println!("Found file: {:?}", file);
/// }
/// ```
///
/// # Panics
///
/// This function will panic if:
/// - The provided `top` path is not a directory.
/// - It fails to read the contents of the directory or any of its subdirectories.
#[must_use]
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
